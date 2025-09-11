#[allow(unused_imports)]
use std::env;
use std::io::Read;
#[allow(unused_imports)]
use std::{io::Write, path::Path};

use anyhow::Ok;
use clap::{ArgGroup, Parser, Subcommand};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use sha1::{Digest, Sha1};
use tokio::fs;
#[derive(Parser)]
#[command(
    //name ="myapp", --version will show name
    version ,
    about = "Git in Rust",
    long_about = "Git in Rust"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}
#[derive(Subcommand, Debug)]
enum Commands {
    /// does testing things
    Init,
    /// hash
    #[command(group(
        ArgGroup::new("input")
            .required(true)              // 必须提供一个输入方式
            .args(&["files", "stdin"])   // 文件模式或 stdin 模式
    ))]
    HashObject {
        #[arg(short = 'w', long = "write")]
        write: bool,

        // /// 输入类型: blob, tree, commit, tag
        // #[arg(short = 't', long = "type", default_value = "blob")]
        // // Clap 默认必填：用户必须提供，否则命令行解析会报错。
        // path: Option<String>,
        /// 文件列表（文件模式）
        #[arg(value_name = "file", num_args = 1.., conflicts_with = "stdin")]
        // 不是 Option，但因为是 Vec，可以为空
        // Clap 默认允许为空 Vec，除非明确设置 num_args = 1.. 或者加 required = true
        files: Vec<String>,

        /// 从标准输入读取内容（stdin 模式）
        #[arg(long = "stdin", conflicts_with = "files")]
        stdin: bool,
    },
    CatFile {
        /// 操作类型: pretty-print, type, size, exists
        #[arg(short = 'p', long = "pretty", conflicts_with_all=&["type", "size", "exists"])]
        pretty: bool,

        #[arg(short = 't', long = "type", conflicts_with_all=&["pretty", "size", "exists"])]
        _object_type: bool,

        #[arg(short = 's', long = "size", conflicts_with_all=&["pretty", "type", "exists"])]
        _size: bool,

        #[arg(short = 'e', long = "exists", conflicts_with_all=&["pretty", "type", "size"])]
        _exists: bool,

        /// 对象 SHA-1 或引用
        object: String,
    },
}
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // You can use print statements as follows for debugging, they'll be visible
    // when running tests.
    // eprintln!("Logs from your program will appear here!");
    let mut args = env::args();
    let _program = args.next(); // 跳过程序名
    let first_arg = args.next(); // 获取用户输入的第一个参数（可能是子命令）
    match Cli::try_parse() {
        Result::Ok(cli) => match &cli.command {
            Some(Commands::Init) => {
                fs::create_dir(".git").await?;
                fs::create_dir(".git/objects").await?;
                fs::create_dir(".git/refs").await?;
                fs::write(".git/HEAD", "ref: refs/heads/main\n").await?;
                println!("Initialized git directory");
            }
            Some(Commands::HashObject {
                write,
                // path,
                files,
                stdin,
            }) => {
                if !*stdin {
                    let results = futures::future::join_all(
                        files.iter().map(|f| hash_and_compress_file(f, *write)),
                    )
                    .await;
                    let output = results
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .map(|(k, _)| k)
                        .collect::<Vec<_>>()
                        .join(" ");

                    println!("{}", output);
                }
            }
            Some(Commands::CatFile {
                pretty,
                _object_type,
                _size,
                _exists,
                object,
            }) => {
                if *pretty {
                    print!(
                        "{}",
                        un_compress_file(format!(
                            ".git/objects/{}/{}",
                            &object[0..2],
                            &object[2..]
                        ))
                        .await?
                    );
                }
            }
            // 这行不会执行，因为默认子命令是必须的，除非使用Some(包装)
            None => println!("No subcommand provided"),
        },
        Err(_) => {
            if let Some(cmd) = first_arg {
                println!("unknown command: {:?}", cmd)
            } else {
                println!("No subcommand provided. Please use `init`");
            }
        }
    }
    Ok(())
}

/// 计算 Git 对象 SHA-1，并返回压缩后的内容
///
/// # 参数
/// - `path`: 文件路径，支持任何 AsRef<Path>
///
/// # 返回
/// - Ok((sha1_hex, compressed_content)):
///   - sha1_hex: 对象 SHA-1 的 16 进制表示
///   - compressed_content: Zlib 压缩后的字节向量
///
/// # 错误
/// - 读取文件或压缩失败会返回 Err(std::io::Error)
async fn hash_and_compress_file<P: AsRef<Path>>(
    path: P,
    write: bool,
) -> Result<(String, Vec<u8>), anyhow::Error> {
    // 1. 读取文件原始内容
    let content = fs::read(path).await?;

    // 2. 压缩内容
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&content)?;
    let compressed = encoder.finish()?;

    // 3. 构造 Git blob header 并计算 SHA-1
    let header = format!("blob {}\0", content.len());
    let store = [header.as_bytes(), &content].concat();

    let mut hasher = Sha1::new();
    hasher.update(&store);
    let sha1_result = hasher.finalize();

    // 4. 转换为 16 进制字符串
    let sha1_hex = hex::encode(sha1_result);

    if write {
        let object_dir = format!(".git/objects/{}", &sha1_hex[0..2]);
        fs::create_dir_all(&object_dir).await?;
        let object_path = format!("{}/{}", object_dir, &sha1_hex[2..]);
        fs::write(object_path, &compressed).await?;
    }

    Ok((sha1_hex, compressed))
}

async fn un_compress_file<P: AsRef<Path>>(path: P) -> Result<String, anyhow::Error> {
    let content = fs::read(path).await?;
    let mut decoder = ZlibDecoder::new(&content[..]);
    let mut ret = String::new();
    decoder.read_to_string(&mut ret)?;
    Ok(ret)
}
