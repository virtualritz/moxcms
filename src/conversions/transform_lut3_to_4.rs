/*
 * // Copyright (c) Radzivon Bartoshyk 3/2025. All rights reserved.
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
use crate::conversions::CompressForLut;
use crate::conversions::interpolator::MultidimensionalInterpolation;
use crate::transform::PointeeSizeExpressible;
use crate::{CmsError, InterpolationMethod, Layout, TransformExecutor};
use num_traits::AsPrimitive;
use std::marker::PhantomData;

pub(crate) struct TransformLut3x4<
    T,
    const LAYOUT: u8,
    const GRID_SIZE: usize,
    const BIT_DEPTH: usize,
> {
    pub(crate) lut: Vec<f32>,
    pub(crate) _phantom: PhantomData<T>,
    pub(crate) interpolation_method: InterpolationMethod,
}

impl<
    T: Copy + AsPrimitive<f32> + Default + CompressForLut + PointeeSizeExpressible,
    const LAYOUT: u8,
    const GRID_SIZE: usize,
    const BIT_DEPTH: usize,
> TransformLut3x4<T, LAYOUT, GRID_SIZE, BIT_DEPTH>
where
    f32: AsPrimitive<T>,
    u32: AsPrimitive<T>,
{
    #[inline(always)]
    fn transform_chunk<'b, Tetrahedral: MultidimensionalInterpolation<'b, GRID_SIZE>>(
        &'b self,
        src: &[T],
        dst: &mut [T],
    ) {
        let cn = Layout::from(LAYOUT);
        let channels = cn.channels();

        let value_scale = ((1 << BIT_DEPTH) - 1) as f32;

        for (src, dst) in src.chunks_exact(channels).zip(dst.chunks_exact_mut(4)) {
            let x = src[cn.r_i()].compress_lut::<BIT_DEPTH>();
            let y = src[cn.g_i()].compress_lut::<BIT_DEPTH>();
            let z = src[cn.b_i()].compress_lut::<BIT_DEPTH>();

            let tetrahedral = Tetrahedral::new(&self.lut);
            let v = tetrahedral.inter4(x, y, z);
            let r = if T::FINITE {
                v * value_scale + 0.5f32
            } else {
                v
            };
            dst[0] = r.v[0].min(value_scale).max(0f32).as_();
            dst[1] = r.v[1].min(value_scale).max(0f32).as_();
            dst[2] = r.v[2].min(value_scale).max(0f32).as_();
            dst[3] = r.v[3].min(value_scale).max(0f32).as_();
        }
    }
}

impl<
    T: Copy + AsPrimitive<f32> + Default + CompressForLut + PointeeSizeExpressible,
    const LAYOUT: u8,
    const GRID_SIZE: usize,
    const BIT_DEPTH: usize,
> TransformExecutor<T> for TransformLut3x4<T, LAYOUT, GRID_SIZE, BIT_DEPTH>
where
    f32: AsPrimitive<T>,
    u32: AsPrimitive<T>,
{
    fn transform(&self, src: &[T], dst: &mut [T]) -> Result<(), CmsError> {
        let cn = Layout::from(LAYOUT);
        let channels = cn.channels();
        if src.len() % channels != 0 {
            return Err(CmsError::LaneMultipleOfChannels);
        }
        if dst.len() % 4 != 0 {
            return Err(CmsError::LaneMultipleOfChannels);
        }
        let src_chunks = src.len() / channels;
        let dst_chunks = dst.len() / 4;
        if src_chunks != dst_chunks {
            return Err(CmsError::LaneSizeMismatch);
        }

        match self.interpolation_method {
            InterpolationMethod::Tetrahedral => {
                use crate::conversions::interpolator::Tetrahedral;
                self.transform_chunk::<Tetrahedral<GRID_SIZE>>(src, dst);
            }
            InterpolationMethod::Pyramid => {
                use crate::conversions::interpolator::Pyramidal;
                self.transform_chunk::<Pyramidal<GRID_SIZE>>(src, dst);
            }
            InterpolationMethod::Prism => {
                use crate::conversions::interpolator::Prismatic;
                self.transform_chunk::<Prismatic<GRID_SIZE>>(src, dst);
            }
            InterpolationMethod::Linear => {
                use crate::conversions::interpolator::Trilinear;
                self.transform_chunk::<Trilinear<GRID_SIZE>>(src, dst);
            }
        }

        Ok(())
    }
}
