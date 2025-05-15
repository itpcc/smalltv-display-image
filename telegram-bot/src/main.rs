use dotenvy::dotenv;
use telegram_bot::{
    photo::{detect_qr, dump_to_bmp, dump_to_png, generate_qr, resize_image},
    telegram::download_photo,
    uploader::upload_image,
};
use teloxide::{prelude::*, types::InputFile};
use tracing_subscriber::EnvFilter;

const DSP_WIDTH: u32 = 240;
const DSP_HEIGHT: u32 = 240;

#[tokio::main]
async fn main() {
    // Tries to load tracing config from environment (RUST_LOG) or uses "debug".
    // load environment variables from .env file
    dotenv().ok();
    tracing_subscriber::fmt()
        .pretty()
        .with_thread_names(true)
        .with_env_filter(EnvFilter::from_env("RUST_LOG"))
        // sets this to be the default, global collector for this application.
        .init();
    tracing::info!("Starting SmallTV QR code bot...");

    let bot = Bot::from_env();

    teloxide::repl(bot, |bot: Bot, msg: Message| async move {
        tracing::info!("Message incoming: {:?}", msg);

        let (unique_id, content, img_qr_ctnt) = if let Some(content) = msg.text() {
            (
                msg.id.to_string(),
                content.to_string(),
                generate_qr(&content.to_string())?,
            )
        } else if let Some(img_nf) = msg.photo() {
            let Some(max_img) = img_nf.iter().max_by(|img_a, img_b| {
                (img_a.height * img_a.width).cmp(&(img_b.height * img_b.width))
            }) else {
                return Ok(());
            };
            let (img_file, tmp_file) = download_photo(&bot, max_img).await?;

            let Some((content, img_qr_ctnt)) = detect_qr(tmp_file.file_path())? else {
                bot.send_message(msg.chat.id, "Not detected QR code")
                    .await?;
                return Ok(());
            };
            (img_file.unique_id.to_owned(), content, img_qr_ctnt)
        } else {
            return Ok(());
        };

        // Final image
        let img_dsp = resize_image(&img_qr_ctnt, DSP_WIDTH, DSP_HEIGHT);

        // Upload file
        let img_bmp_vec = dump_to_bmp(&img_dsp)?;
        if let Err(e) = upload_image(&unique_id, &img_bmp_vec).await {
            bot.send_message(msg.chat.id, format!("Unable to Upload: {}", e))
                .await?;
            tracing::error!("Upload error: {:?}", e);

            return Ok(());
        };

        // Send image back
        let img_png_vec = dump_to_png(&img_dsp)?;
        let img_tx = InputFile::memory(img_png_vec).file_name(format!("{}.png", unique_id));
        bot.send_photo(msg.chat.id, img_tx)
            .caption(content.clone())
            .await?;

        Ok(())
    })
    .await;
}
