use std::{
    io::{self, Cursor, ErrorKind},
    path::PathBuf,
    sync::Arc,
};

use image::{DynamicImage, ImageFormat, ImageReader, Luma, imageops::FilterType};
use qrcode::{EcLevel, QrCode};
use teloxide::prelude::ResponseResult;

pub fn generate_qr(payload: &String) -> ResponseResult<image::DynamicImage> {
    // Encode some data into bits.
    let code = if
    // PrompyPay
    payload.starts_with("000201") ||
        // Bill payment
        payload.starts_with("|")
    {
        QrCode::with_error_correction_level(payload.as_bytes(), EcLevel::L)
    } else {
        QrCode::new(payload.as_bytes())
    }
    .map_err(|e| Arc::new(io::Error::new(io::ErrorKind::InvalidData, e)))?;

    // Render the bits into an image.
    let image = code.render::<Luma<u8>>().build();

    Ok(image::DynamicImage::ImageLuma8(image))
}

pub fn detect_qr(path: &PathBuf) -> ResponseResult<Option<(String, DynamicImage)>> {
    // Attempt to decode image
    let img = ImageReader::open(path)
        .map_err(Arc::new)?
        .with_guessed_format()
        .map_err(Arc::new)?
        .decode()
        .map_err(|e| Arc::new(io::Error::new(ErrorKind::Unsupported, e)))?;
    let img_luma = img.to_luma8();
    let mut img_qr = rqrr::PreparedImage::prepare(img_luma);
    // Search for grids, without decoding
    let grids = img_qr.detect_grids();

    if grids.is_empty() {
        return Ok(None);
    }

    let qr_bound = grids[0].bounds;

    tracing::debug!("Message bounds: {:?}", grids[0].bounds);
    // Decode the grid
    let Ok((meta, content)) = grids[0].decode() else {
        return Ok(None);
    };
    tracing::debug!("Message meta: {:?}", meta);

    // Generate picture to display
    // Note that BORA ThaiD require to use the QR with their logo shown
    let img_qr_ctnt = if content.to_lowercase().contains("bora.dopa") {
        let min_x = qr_bound.iter().map(|p| p.x).min().unwrap_or(0);
        let min_y = qr_bound.iter().map(|p| p.y).min().unwrap_or(0);
        let max_x = qr_bound.iter().map(|p| p.x).max().unwrap_or(0);
        let max_y = qr_bound.iter().map(|p| p.y).max().unwrap_or(0);
        img.crop_imm(
            0i32.max(min_x - 5) as u32,
            0i32.max(min_y - 5) as u32,
            (max_x + 5).abs_diff(min_x),
            (max_y + 5).abs_diff(min_y),
        )
    } else {
        generate_qr(&content)?
    };
    Ok(Some((content, img_qr_ctnt)))
}

pub fn resize_image(img: &image::DynamicImage, width: u32, height: u32) -> DynamicImage {
    img.resize(width, height, FilterType::Gaussian)
}

fn dump_to(img: &image::DynamicImage, fmt: ImageFormat) -> ResponseResult<Vec<u8>> {
    let mut img_vec = Vec::<u8>::new();
    img.write_to(&mut Cursor::new(&mut img_vec), fmt)
        .map_err(|e| Arc::new(io::Error::new(io::ErrorKind::Other, e)))?;

    Ok(img_vec)
}

pub fn dump_to_bmp(img: &image::DynamicImage) -> ResponseResult<Vec<u8>> {
    dump_to(img, ImageFormat::Bmp)
}

pub fn dump_to_png(img: &image::DynamicImage) -> ResponseResult<Vec<u8>> {
    dump_to(img, ImageFormat::Png)
}
