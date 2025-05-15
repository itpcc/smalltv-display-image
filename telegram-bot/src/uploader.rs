use std::{env, sync::Arc};

use russh::client;
use russh_sftp::{client::SftpSession, protocol::OpenFlags};
use tokio::io::AsyncWriteExt;

struct Client;

impl russh::client::Handler for Client {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        tracing::debug!("check_server_key: {:?}", server_public_key);
        Ok(true)
    }

    async fn data(
        &mut self,
        channel: russh::ChannelId,
        data: &[u8],
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        tracing::debug!("data on channel {:?}: {}", channel, data.len());
        Ok(())
    }
}

pub async fn upload_image(name: &String, data: &[u8]) -> anyhow::Result<bool> {
    let config = client::Config::default();
    let sh = Client {};
    let mut session = client::connect(
        Arc::new(config),
        (
            env::var("SFTP_HOST")?,
            env::var("SFTP_PORT")?.parse::<u16>()?,
        ),
        sh,
    )
    .await?;
    let auth_res = session
        .authenticate_password(env::var("SFTP_USERNAME")?, env::var("SFTP_PASSWORD")?)
        .await?;
    if !auth_res.success() {
        tracing::error!("Unable to connect: {:#?}", auth_res);
        return Ok(false);
    }

    let channel = session.channel_open_session().await?;

    channel.request_subsystem(true, "sftp").await?;

    let sftp = SftpSession::new(channel.into_stream()).await?;
    let data_dir = env::var("SFTP_PATH")?;
    tracing::info!("current path: {:?}", sftp.canonicalize(".").await?);

    // * Write image file
    let mut file = sftp
        .open_with_flags(
            format!("{}/qr-{}.bmp", data_dir, name),
            OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
        )
        .await?;
    file.write_all(data).await?;
    file.sync_all().await?;
    file.shutdown().await?;

    // * Update current file indicator
    let mut file = sftp
        .open_with_flags(
            format!("{}/latest_version", data_dir),
            OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
        )
        .await?;
    file.write_all(name.as_bytes()).await?;
    file.sync_all().await?;
    file.shutdown().await?;

    Ok(true)
}
