use image::{ImageBuffer, ImageFormat, Rgb};
use std::io::Cursor;

#[allow(dead_code)]
pub async fn image_to_buffer(img: ImageBuffer<Rgb<u8>, Vec<u8>>) -> Vec<u8> {
    let mut data = Vec::new();
    {
        let dynamic_image = image::DynamicImage::ImageRgb8(img);
        dynamic_image.write_to(&mut Cursor::new(&mut data), ImageFormat::Png).expect("Failed to encode image");
    }
    data
}
