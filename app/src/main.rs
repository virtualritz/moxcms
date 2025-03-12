/*
 * // Copyright (c) Radzivon Bartoshyk 2/2025. All rights reserved.
 * //
 * // Redistribution and use in source and binary forms, with or without modification,
 * // are permitted provided that the following conditions are met:
 * //
 * // 1.  Redistributions of source code must retain the above copyright notice, this
 * // list of conditions and the following disclaimer.
 * //
 * // 2.  Redistributions in binary form must reproduce the above copyright notice,
 * // this list of conditions and the following disclaimer in the documentation
 * // and/or other materials provided with the distribution.
 * //
 * // 3.  Neither the name of the copyright holder nor the names of its
 * // contributors may be used to endorse or promote products derived from
 * // this software without specific prior written permission.
 * //
 * // THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * // AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * // IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
 * // DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
 * // FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * // DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
 * // SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
 * // CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * // OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
 * // OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
 */
use image::GenericImageView;
use lcms2::{Intent, PixelFormat, Profile, Transform};
use moxcms::{ColorProfile, Layout, RenderingIntent, TransformOptions};
use std::fs;
use std::fs::File;
use std::io::{BufReader, Read};
use std::time::Instant;
use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

fn main() {
    let funny_icc = fs::read("./assets/srgb_perceptual.icc").unwrap();
    let funny_profile = ColorProfile::new_from_slice(&funny_icc).unwrap();

    let encoded = funny_profile.encode().unwrap();

    let decoded = ColorProfile::new_from_slice(&encoded).unwrap();

    // println!("{:?}", decoded);

    fs::write("./bt2020.icc", encoded).unwrap();

    let srgb_perceptual_icc = fs::read("./assets/srgb_perceptual.icc").unwrap();
    let srgb_perceptual_profile = ColorProfile::new_from_slice(&srgb_perceptual_icc).unwrap();

    let f_str = "./assets/pq_test.jpg";
    let file = File::open(f_str).expect("Failed to open file");

    let img = image::ImageReader::open(f_str).unwrap().decode().unwrap();
    let rgb = img.to_rgb8();

    let reader = BufReader::new(file);
    let ref_reader = &reader;

    let options = DecoderOptions::new_fast().jpeg_set_out_colorspace(ColorSpace::RGB);

    let mut decoder = JpegDecoder::new_with_options(reader, options);

    // let mut decoder = JpegDecoder::new(reader);
    decoder.options().set_use_unsafe(true);
    decoder.decode_headers().unwrap();
    let mut real_dst = vec![0u8; decoder.output_buffer_size().unwrap()];

    let custom_profile = Profile::new_icc(&srgb_perceptual_icc).unwrap();
    //
    let srgb_profile = Profile::new_srgb();

    decoder.decode_into(&mut real_dst).unwrap();

    let real_dst = real_dst
        .chunks_exact(3)
        .flat_map(|x| [x[0], x[1], x[2], 255u8])
        .map(|x| (x as u16) << 8 | (x as u16))
        .collect::<Vec<_>>();

    // let t1 = Transform::new(
    //     &custom_profile,
    //     PixelFormat::RGB_8,
    //     &srgb_profile,
    //     PixelFormat::RGB_8,
    //     Intent::Perceptual,
    // )
    // .unwrap();

    // let t2 = Transform::new(
    //     &custom_profile,
    //     PixelFormat::Lab_8,
    //     &srgb_profile,
    //     PixelFormat::RGB_8,
    //     Intent::Perceptual,
    // )
    //     .unwrap();

    let mut cmyk = vec![0u8; decoder.output_buffer_size().unwrap()];

    // t1.transform_pixels(&real_dst, &mut cmyk);

    let icc = decoder.icc_profile().unwrap();
    let color_profile = ColorProfile::new_from_slice(&funny_icc).unwrap();
    let cmyk_profile = ColorProfile::new_from_slice(&funny_icc).unwrap();
    // let color_profile = ColorProfile::new_gray_with_gamma(2.2);
    let mut dest_profile = ColorProfile::new_srgb();

    // // let instant = Instant::now();
    // let rgb_to_cmyk = dest_profile
    //     .create_transform_8bit(
    //         Layout::Rgb,
    //         &cmyk_profile,
    //         Layout::Rgba,
    //         TransformOptions {
    //             rendering_intent: RenderingIntent::Perceptual,
    //         },
    //     )
    //     .unwrap();
    //
    // rgb_to_cmyk.transform(&real_dst, &mut cmyk).unwrap();

    // println!("Execution time: {:?}", instant.elapsed());

    // let t = Transform::new(&srgb_profile, PixelFormat::RGB_8, &custom_profile, PixelFormat::RGB_8, Intent::Perceptual).unwrap();

    dest_profile.rendering_intent = RenderingIntent::Perceptual;
    let transform = color_profile
        .create_transform_16bit(
            Layout::Rgba,
            &dest_profile,
            Layout::Rgba,
            TransformOptions {
                rendering_intent: RenderingIntent::Perceptual,
            },
        )
        .unwrap();
    let mut dst = vec![0u16; real_dst.len()];
    // t.transform_pixels(&real_dst)

    // let gray_image = rgb
    //     .chunks_exact(3)
    //     .map(|chunk| {
    //         (0.2126 * chunk[0] as f32 + 0.7152 * chunk[1] as f32 + 0.0722 * chunk[2] as f32).round()
    //             as u8
    //     })
    //     .collect::<Vec<u8>>();
    //
    let instant = Instant::now();
    for (src, dst) in real_dst
        .chunks_exact(img.width() as usize * 4)
        .zip(dst.chunks_exact_mut(img.width() as usize * 4))
    {
        transform
            .transform(
                &src[..img.width() as usize * 4],
                &mut dst[..img.width() as usize * 4],
            )
            .unwrap();
    }

    // t2.transform_pixels(&cmyk, &mut dst);

    println!("Estimated time: {:?}", instant.elapsed());

    // let image = JxlImage::builder()
    //     .pool(JxlThreadPool::none())
    //     .read(std::io::Cursor::new(fs::read("./assets/test.jxl").unwrap()))
    //     .unwrap();
    //
    // let render = image.render_frame(0).unwrap();
    // let rendered_icc = image.rendered_icc();
    // let image = render.image_all_channels();
    // let float_image = RgbImage::from_raw(
    //     image.width() as u32,
    //     image.height() as u32,
    //     image
    //         .buf()
    //         .iter()
    //         .map(|x| x * 255. + 0.5)
    //         .map(|x| x as u8)
    //         .collect::<Vec<_>>(),
    // )
    //     .unwrap();
    //
    // let jxl_profile = ColorProfile::new_from_slice(&rendered_icc).unwrap();
    // let mut dst2 = vec![0u8; float_image.len()];
    // let transform2 = jxl_profile
    //     .create_transform_8bit(&dest_profile, Layout::Rgb8)
    //     .unwrap();
    //
    // for (src, dst) in float_image
    //     .chunks_exact(float_image.width() as usize * 3)
    //     .zip(dst2.chunks_exact_mut(float_image.dimensions().0 as usize * 3))
    // {
    //     // ot.transform_pixels(src, dst);
    //
    //     transform2
    //         .transform(
    //             &src[..float_image.dimensions().0 as usize * 3],
    //             &mut dst[..float_image.dimensions().0 as usize * 3],
    //         )
    //         .unwrap();
    // }
    //
    // image::save_buffer(
    //     "jx.jpg",
    //     &dst2,
    //     float_image.dimensions().0,
    //     float_image.dimensions().1,
    //     image::ExtendedColorType::Rgb8,
    // )
    // .unwrap();

    let dst = dst.iter().map(|&x| (x >> 8) as u8).collect::<Vec<_>>();

    image::save_buffer(
        "v_new_sat.png",
        &dst,
        img.dimensions().0,
        img.dimensions().1,
        image::ExtendedColorType::Rgba8,
    )
    .unwrap();
}

// fn main() {
//     let us_swop_icc = fs::read("./assets/us_swop_coated.icc").unwrap();
//
//     let width = 5000;
//     let height = 5000;
//
//     let cmyk = vec![0u8; width * height * 4];
//
//     let color_profile = ColorProfile::new_from_slice(&us_swop_icc).unwrap();
//     let dest_profile = ColorProfile::new_srgb();
//     let mut dst = vec![0u8; width * height * 4];
//     let transform = color_profile
//         .create_transform_8bit(
//             Layout::Rgba,
//             &dest_profile,
//             Layout::Rgba,
//             TransformOptions::default(),
//         )
//         .unwrap();
//     transform.transform(&cmyk, &mut dst).unwrap();
// }
