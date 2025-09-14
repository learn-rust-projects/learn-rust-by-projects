use std::{
    env,
    ffi::CStr,
    io::{BufRead, Cursor, Read, Write},
    path::{Path, PathBuf},
};

use anyhow::Context;
use clap::{ArgGroup, Parser, Subcommand};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use futures::io::Empty;
use sha1::{Digest, Sha1};
use tempfile::NamedTempFile;
use tokio::fs;

use crate::{
    commands,
    objects::{Kind, Object, hash_to_reader},
};
pub(crate) async fn invoke_commit_tree(
    tree_sha: String,
    message: String,
    parent: Option<String>,
) -> Result<[u8; 20], anyhow::Error> {
    let mut buf = Vec::new();
    writeln!(buf, "tree {tree_sha}")?;
    if let Some(parent) = parent {
        writeln!(buf, "parent {parent}")?;
    }
    let name = "Levio-Z";
    let email = "67247011+Levio-z@users.noreply.github.com";
    let time = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .context("current system time is before UNIX epoch")?
        .as_secs();
    writeln!(buf, "author {name} <{email}> {time} +0800")?;
    writeln!(buf, "committer {name} <{email}> {time} +0800")?;
    writeln!(buf)?;
    writeln!(buf, "{message}")?;

    let mut commit = Object {
        kind: Kind::Commit,
        expected_size: buf.len() as u64,
        reader: Cursor::new(buf),
    };
    let hash = commit.write_object().await?;
    println!("{}", hex::encode(hash));
    Ok(hash)
}

pub(crate) async fn invoke_commit(message: String) -> Result<(), anyhow::Error> {
    let head_ref = std::fs::read_to_string(".git/HEAD").context("read HEAD")?;
    let Some(head_ref) = head_ref.strip_prefix("ref: ") else {
        anyhow::bail!("refusing to commit onto detached HEAD");
    };
    // 去除末尾的换行符
    let head_ref = head_ref.trim_end();
    let parent = if let Ok(hash) = std::fs::read_to_string(format!(".git/{head_ref}")) {
        Some(hash.trim().to_string())
    } else {
        None
    };

    // 计算hash
    let tree_hash = crate::commands::write_tree::invoke(PathBuf::from("."))
        .await
        .context("write tree")?;

    // 提交hash
    let commit_hash = invoke_commit_tree(hex::encode(tree_hash), message, parent)
        .await
        .context("commit tree")?;
    let commit_hash = hex::encode(commit_hash);
    std::fs::write(format!(".git/{head_ref}"), &commit_hash)
        .with_context(|| format!("update HEAD reference target {head_ref}"))?;
    println!("HEAD is now at {commit_hash}");
    Ok(())
}
