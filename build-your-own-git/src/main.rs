#[allow(unused_imports)]
use std::env;
use std::{
    ffi::CStr,
    io::{BufRead, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::{ArgGroup, Parser, Subcommand};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use sha1::{Digest, Sha1};
use tempfile::NamedTempFile;
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
        files: Vec<PathBuf>,

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
        Result::Ok(cli) => match cli.command {
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
                if !stdin {
                    let results = futures::future::join_all(
                        files.iter().map(|f| hash_and_compress_file(f, write)),
                    )
                    .await;
                    let output = results
                        .into_iter()
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
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
                anyhow::ensure!(
                    pretty,
                    "mode must be given without -p ,and we don't support mode"
                );
                if pretty {
                    decompress_file(format!(".git/objects/{}/{}", &object[0..2], &object[2..]))
                        .await?
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
/// - Ok(sha1_hex):
///   - sha1_hex: 对象 SHA-1 的 16 进制表示
///
/// # 错误
/// - 读取文件或压缩失败会返回 Err(std::io::Error)
async fn hash_and_compress_file(path: &PathBuf, write: bool) -> Result<String, anyhow::Error> {
    // 使用tempfile crate创建临时文件
    let temp_file = NamedTempFile::new()?;
    let temp_path = temp_file.path().to_path_buf();

    // 获取文件元数据
    let stat = fs::metadata(path)
        .await
        .with_context(|| format!("stat file metadata failed: {}", path.display()))?;

    // 构建writer ，writer 是一个压缩写入器，压缩后的内容会写入到临时文件中
    let writer = ZlibEncoder::new(temp_file, Compression::default());
    // 使用HashWriter 包装writer，HashWriter 会计算写入的内容的hash
    let mut writer = HashWriter {
        writer,
        hasher: Sha1::new(),
    };

    // 1. 构造 Git blob header 并计算 SHA-1
    write!(writer, "blob {}\0", stat.len())?;

    // 2. 读取文件原始内容
    let mut file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    std::io::copy(&mut file, &mut writer).context("stream file into blob")?;

    // 3. 计算hash和压缩，hash是和压缩一起进行的
    let _ = writer.writer.finish()?;
    let hex_sha1 = hex::encode(writer.hasher.finalize());

    // 4. 为什么需要临时文件，因为压缩后的文件名称和地址是根据 hash
    // 计算的，所以需要先压缩到临时文件，
    if write {
        fs::create_dir_all(format!(".git/objects/{}/", &hex_sha1[..2])).await?;
        std::fs::rename(
            temp_path,
            format!(".git/objects/{}/{}", &hex_sha1[..2], &hex_sha1[2..]),
        )
        .context("move blob file into .git/objects")?;
    }

    Ok(hex_sha1)
}
// 解压缩文件
async fn decompress_file<P: AsRef<Path>>(path: P) -> Result<(), anyhow::Error> {
    let f = std::fs::File::open(path).context("open in .git/objects")?;
    let decoder = ZlibDecoder::new(f);
    let mut buf = std::io::BufReader::new(decoder);

    let mut ret = Vec::new();
    // 1. 读取文件头
    buf.read_until(b'\0', &mut ret)?;
    // let s = std::str::from_utf8(&ret).unwrap();
    let c_str = CStr::from_bytes_with_nul(&ret).expect("Invalid C string");
    let header = c_str
        .to_str()
        .context(" .git/objects file header isn't valid utf-8")?;
    // 使用split_once 而不是split 是为了避免文件名中包含空格
    let Some((kind, size)) = header.split_once(' ') else {
        anyhow::bail!(".git/objects file header did not start with a konw type {header}");
    };
    // 处理类型
    let kind = match kind {
        "blob" => Kind::Blob,
        _ => anyhow::bail!("we do not know how to print a '{kind}'"),
    };

    // 要得到 usize，必须显式解析：
    let size = size
        .parse::<u64>()
        .context(" .git/objects file header size isn't valid:{size}")?;
    let mut buf = buf.take(size);
    let mut stdout = std::io::stdout().lock();
    // 直接使用std::io::copy
    match kind {
        Kind::Blob => {
            let n = std::io::copy(&mut buf, &mut stdout)?;
            anyhow::ensure!(
                n == size,
                ".git/objects file was not be expected size (expected {size}, got {n})"
            );
        }
        _ => anyhow::bail!("we do not know how to print a '{kind:?}'"),
    };
    Ok(())
}

#[derive(Debug)]
enum Kind {
    Blob,
    _Tree,
    _Commit,
    _Tag,
}

struct HashWriter<W> {
    writer: W,
    hasher: Sha1,
}

impl<W> Write for HashWriter<W>
where
    W: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let n = self.writer.write(buf)?;
        self.hasher.update(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
