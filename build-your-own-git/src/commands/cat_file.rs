use std::{
    env,
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

use crate::objects::{Kind, hash_to_reader};

pub(crate) async fn invoke(path: &str, pretty: bool) -> Result<(), anyhow::Error> {
    anyhow::ensure!(
        pretty,
        "mode must be given without -p ,and we don't support mode"
    );
    let mut hash_object = hash_to_reader(path).await?;
    let mut stdout: std::io::StdoutLock<'static> = std::io::stdout().lock();
    // 直接使用std::io::copy将内容输出到终端
    match hash_object.kind {
        Kind::Blob => {
            let n = std::io::copy(&mut hash_object.reader, &mut stdout)?;
            anyhow::ensure!(
                n == hash_object.expected_size,
                ".git/objects file was not be expected size (expected {0}, got {n})",
                hash_object.expected_size
            );
        }
        _ => anyhow::bail!("we do not know how to print a '{:?}'", hash_object.kind),
    };
    Ok(())
}
