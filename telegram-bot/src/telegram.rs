use std::{
    io::{Error, ErrorKind, SeekFrom},
    sync::Arc,
};

use async_tempfile::TempFile;
use teloxide::{
    Bot,
    net::Download,
    prelude::{Requester, ResponseResult},
    types::{File, PhotoSize},
};
use tokio::io::AsyncSeekExt;

pub async fn download_photo(bot: &Bot, photo: &PhotoSize) -> ResponseResult<(File, TempFile)> {
    let mut tmp_file = TempFile::new().await.map_err(|e| {
        Arc::new(match e {
            async_tempfile::Error::InvalidDirectory => Error::new(ErrorKind::PermissionDenied, e),
            async_tempfile::Error::InvalidFile => Error::new(ErrorKind::InvalidData, e),
            async_tempfile::Error::Io(error) => error,
        })
    })?;
    let img_file = bot.get_file(&photo.file.id).await?;
    bot.download_file(&img_file.path, &mut tmp_file).await?;
    tracing::debug!("img_file: {:?}", img_file);
    tmp_file.seek(SeekFrom::Start(0)).await.map_err(Arc::new)?;

    Ok((img_file, tmp_file))
}
