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
// use jxl_oxide::{JxlImage, JxlThreadPool, Lcms2, Moxcms};
use lcms2::{Intent, PixelFormat, Profile, Transform};
use moxcms::{ColorProfile, InterpolationMethod, Layout, RenderingIntent, TransformOptions};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::time::Instant;
use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

fn compute_abs_diff4(src: &[f32], dst: &[[f32; 4]], highlights: &mut [f32]) {
    let mut abs_r = f32::MIN;
    let mut abs_g = f32::MIN;
    let mut abs_b = f32::MIN;
    let mut abs_a = f32::MIN;
    let mut mean_r = 0f32;
    let mut mean_g = 0f32;
    let mut mean_b = 0f32;
    for ((src, dst), h) in src
        .chunks_exact(4)
        .zip(dst.iter())
        .zip(highlights.chunks_exact_mut(4))
    {
        let dr = (src[0] - dst[0]).abs();
        abs_r = dr.max(abs_r);
        mean_r += dr.abs();
        abs_g = (src[1] - dst[1]).abs().max(abs_g);
        mean_g += (src[1] - dst[1]).abs();
        abs_b = (src[2] - dst[2]).abs().max(abs_b);
        mean_b += (src[2] - dst[2]).abs();
        abs_a = (src[3] - dst[3]).abs().max(abs_a);
        if dr > 0.1 {
            h[0] = 1.0f32;
            h[3] = 1.0f32;
        } else if dr < 0.2 {
            h[1] = 1.0f32;
            h[3] = 1.0f32;
        }
    }
    mean_r /= dst.len() as f32;
    mean_g /= dst.len() as f32;
    mean_b /= dst.len() as f32;
    println!("Abs R {} Mean R {}", abs_r, mean_r);
    println!("Abs G {} Mean G {}", abs_g, mean_g);
    println!("Abs B {} Mean G {}", abs_b, mean_b);
    println!("Abs A {}", abs_a);
}

fn compute_abs_diff42(src: &[f32], dst: &[f32]) {
    let mut abs_r = f32::MIN;
    let mut abs_g = f32::MIN;
    let mut abs_b = f32::MIN;
    let mut abs_a = f32::MIN;
    let mut mean_r = 0f32;
    let mut mean_g = 0f32;
    let mut mean_b = 0f32;
    for (src, dst) in src.chunks_exact(4).zip(dst.chunks_exact(4)) {
        let dr = (src[0] - dst[0]).abs();
        abs_r = dr.max(abs_r);
        mean_r += dr.abs();
        abs_g = (src[1] - dst[1]).abs().max(abs_g);
        mean_g += (src[1] - dst[1]).abs();
        abs_b = (src[2] - dst[2]).abs().max(abs_b);
        mean_b += (src[2] - dst[2]).abs();
        abs_a = (src[3] - dst[3]).abs().max(abs_a);
    }
    mean_r /= dst.len() as f32;
    mean_g /= dst.len() as f32;
    mean_b /= dst.len() as f32;
    println!("Abs R {} Mean R {}", abs_r, mean_r);
    println!("Abs G {} Mean G {}", abs_g, mean_g);
    println!("Abs B {} Mean G {}", abs_b, mean_b);
    println!("Abs A {}", abs_a);
}

fn main() {
    let funny_icc = fs::read("./assets/CGATS21_CRPC5.icc").unwrap();

    // println!("{:?}", decoded);

    let srgb_perceptual_icc = fs::read("./assets/srgb_perceptual.icc").unwrap();

    let funny_profile = ColorProfile::new_from_slice(&funny_icc).unwrap();

    let srgb_perceptual_profile = ColorProfile::new_from_slice(&srgb_perceptual_icc).unwrap();
    let out_profile = ColorProfile::new_srgb();

    let f_str = "./assets/bench.jpg";
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
        .map(|x| x as f32 / 255.0)
        .collect::<Vec<_>>();

    let pr1 = lcms2::Profile::new_icc(&funny_icc).unwrap();

    let t1 = Transform::new(
        &lcms2::Profile::new_srgb(),
        PixelFormat::RGBA_FLT,
        &pr1,
        PixelFormat::CMYK_FLT,
        Intent::Perceptual,
    )
    .unwrap();

    let t2 = Transform::new(
        &pr1,
        PixelFormat::CMYK_FLT,
        &lcms2::Profile::new_srgb(),
        PixelFormat::RGBA_FLT,
        Intent::Perceptual,
    )
    .unwrap();

    let mut cmyk = vec![0f32; (decoder.output_buffer_size().unwrap() / 3) * 4];

    let mut cmyk_lcms2 = vec![[0f32; 4]; (decoder.output_buffer_size().unwrap() / 3) * 4];

    let icc = decoder.icc_profile().unwrap();
    let color_profile = ColorProfile::new_from_slice(&srgb_perceptual_icc).unwrap();
    let cmyk_profile = ColorProfile::new_from_slice(&funny_icc).unwrap();
    // let color_profile = ColorProfile::new_gray_with_gamma(2.2);
    let mut dest_profile = ColorProfile::new_srgb();

    // t1.transform_pixels(&real_dst, &mut cmyk);

    let lcms2_src = real_dst
        .chunks_exact(4)
        .map(|x| [x[0], x[1], x[2], x[3]])
        .collect::<Vec<_>>();

    t1.transform_pixels(lcms2_src.as_slice(), cmyk_lcms2.as_mut_slice());

    let time = Instant::now();

    let transform = dest_profile
        .create_transform_f32(
            Layout::Rgba,
            &funny_profile,
            Layout::Rgba,
            TransformOptions {
                rendering_intent: RenderingIntent::Perceptual,
                allow_use_cicp_transfer: false,
                prefer_fixed_point: false,
                interpolation_method: InterpolationMethod::Tetrahedral,
                black_point_compensation: false,
            },
        )
        .unwrap();

    transform.transform(&real_dst, &mut cmyk).unwrap();

    let time = Instant::now();

    let transform = funny_profile
        .create_transform_f32(
            Layout::Rgba,
            &out_profile,
            Layout::Rgba,
            TransformOptions {
                rendering_intent: RenderingIntent::Perceptual,
                allow_use_cicp_transfer: false,
                prefer_fixed_point: false,
                interpolation_method: InterpolationMethod::Tetrahedral,
                black_point_compensation: false,
            },
        )
        .unwrap();
    println!("Rendering took {:?}", time.elapsed());
    let mut dst = vec![0f32; real_dst.len()];

    let mut v_max = f32::MIN;

    for src in cmyk_lcms2.iter() {
        for &src in src.iter() {
            v_max = src.max(v_max);
        }
    }

    // v_max = 1.;
    //
    // let instant = Instant::now();

    let clms = cmyk_lcms2
        .iter()
        .flat_map(|x| [x[0] / v_max, x[1] / v_max, x[2] / v_max, x[3] / v_max])
        .collect::<Vec<_>>();

    for (src, dst) in clms
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

    dst = dst
        .chunks_exact(4)
        .flat_map(|x| [x[0], x[1], x[2], 1.])
        .collect();

    let mut rgba_lcms2 = vec![[0f32; 4]; (decoder.output_buffer_size().unwrap() / 3)];

    t2.transform_pixels(&cmyk_lcms2, &mut rgba_lcms2);

    let mut highlights = vec![0f32; dst.len()];

    compute_abs_diff4(&dst, &rgba_lcms2, &mut highlights);

    // println!("Estimated time: {:?}", instant.elapsed());

    // let mut image = JxlImage::builder()
    //     .pool(JxlThreadPool::none())
    //     .read(std::io::Cursor::new(
    //         fs::read("./assets/input(1).jxl").unwrap(),
    //     ))
    //     .unwrap();
    // image.set_cms(Moxcms);
    //
    // let render = image.render_frame(0).unwrap();
    // let rendered_icc = image.rendered_icc();
    // let image = render.image_all_channels();
    // let img_buf = image.buf();
    //
    // let real_img_mox = img_buf
    //     .chunks_exact(5)
    //     .flat_map(|x| [x[0], x[1], x[2], x[3]])
    //     .collect::<Vec<_>>();
    //
    // let mut image = JxlImage::builder()
    //     .pool(JxlThreadPool::none())
    //     .read(std::io::Cursor::new(
    //         fs::read("./assets/input(1).jxl").unwrap(),
    //     ))
    //     .unwrap();
    // image.set_cms(Lcms2);
    //
    // let real_img_lcms = img_buf
    //     .chunks_exact(5)
    //     .flat_map(|x| [x[0], x[1], x[2], x[3]])
    //     .collect::<Vec<_>>();
    //
    // compute_abs_diff42(&real_img_mox, &real_img_lcms);
    //
    // let real_img_data = img_buf
    //     .chunks_exact(5)
    //     .flat_map(|x| [x[0], x[1], x[2], x[3]])
    //     .map(|x| (x * 255.0 + 0.5) as u8)
    //     .collect::<Vec<_>>();
    //
    // let jxl_profile = ColorProfile::new_from_slice(&rendered_icc).unwrap();

    // let mut dst2 = vec![0u8; real_img_data.len()];
    // let transform2 = jxl_profile
    //     .create_transform_8bit(
    //         Layout::Rgba,
    //         &dest_profile,
    //         Layout::Rgba,
    //         TransformOptions::default(),
    //     )
    //     .unwrap();
    //
    // for (src, dst) in real_img_data
    //     .chunks_exact(img.width() as usize * 4)
    //     .zip(dst2.chunks_exact_mut(image.width() as usize * 4))
    // {
    //     // ot.transform_pixels(src, dst);
    //
    //     transform2
    //         .transform(
    //             &src[..image.width() as usize * 4],
    //             &mut dst[..image.height() as usize * 4],
    //         )
    //         .unwrap();
    // }
    //
    // image::save_buffer(
    //     "moxcms.png",
    //     &real_img_data,
    //     image.width() as u32,
    //     image.height() as u32,
    //     image::ExtendedColorType::Rgba8,
    // )
    // .unwrap();

    // let dst = dst.chunks_exact(4).map(|x| {
    //     [x[0], x[1], x[2], 255]
    // }).flat_map(|x| x).collect::<Vec<u8>>();

    let dst = dst
        .iter()
        .map(|&x| (x * 255f32).round() as u8)
        .collect::<Vec<_>>();
    image::save_buffer(
        "v_new_dst.png",
        &dst,
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
    )
    .unwrap();

    let dst = highlights
        .iter()
        .map(|&x| (x * 255f32).round() as u8)
        .collect::<Vec<_>>();
    image::save_buffer(
        "v_new_satu16.png",
        &dst,
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgba8,
    )
    .unwrap();

    let dst = rgba_lcms2
        .iter()
        .flat_map(|x| [x[0], x[1], x[2], 1.])
        .map(|x| (x * 255f32).round() as u8)
        .collect::<Vec<_>>();
    image::save_buffer(
        "v_new_lcms2.png",
        &dst,
        img.width(),
        img.height(),
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
//     let mut dst = vec![32u8; width * height * 4];
//     let r= rand::rng().random_range(0..255) as u8;
//     let g= rand::rng().random_range(0..255) as u8;
//     let b= rand::rng().random_range(0..255) as u8;
//     for dst in dst.chunks_exact_mut(4) {
//         dst[0] =r ;
//         dst[1] = g ;
//         dst[2] = b;
//         dst[3] = 255;
//     }
//     let transform = color_profile
//         .create_transform_8bit(
//             Layout::Rgba,
//             &dest_profile,
//             Layout::Rgba,
//             TransformOptions {
//                 interpolation_method: InterpolationMethod::Prism,
//                 ..Default::default()
//             },
//         )
//         .unwrap();
//     transform.transform(&cmyk, &mut dst).unwrap();
// }
