use crate::JpegImage;
use crate::pixel_matrix::pixel_matrix::PixelMatrix;
use std::thread;

impl JpegImage {
    pub fn get_downsampling_factor(downsampling_ratio: (u8, u8, u8)) -> (usize, usize) {
        // returns the horizontal and vertical factors by which the chrominance channels must be downsampled
        match downsampling_ratio {
            (4, 4, 4) => {
                return (1, 1);
            }
            (4, 2, 0) => {
                return (2, 2);
            }
            (4, 2, 2) => {
                return (2, 1);
            }
            _ => {
                panic!("Invalid chrominance downsampling ratio!");
            }
        }
    }

    pub fn get_downsampled_dimensions(
        width: usize,
        height: usize,
        horizontal_downsampling: usize,
        vertical_downsampling: usize
    ) -> (usize, usize) {
        let aux_width = width / horizontal_downsampling;
        let downsampled_width = if aux_width % 8 == 0 {
            aux_width as usize
        } else {
            (aux_width + 8 - (aux_width % 8)) as usize
        };

        let height_aux = height / vertical_downsampling;
        let downsampled_height = if height_aux % 8 == 0 {
            height_aux as usize
        } else {
            (height_aux + 8 - (height_aux % 8)) as usize
        };
        (downsampled_width, downsampled_height)
    }

    pub fn chrominance_downsampling(&mut self) {
        let (horizontal_downsampling, vertical_downsampling): (
            usize,
            usize,
        ) = Self::get_downsampling_factor(self.chrominance_downsampling_ratio);

        if horizontal_downsampling == 1 && vertical_downsampling == 1 {
            return;
        }

        let (downsampled_width, downsampled_height) = Self::get_downsampled_dimensions(
            self.width as usize,
            self.height as usize,
            horizontal_downsampling,
            vertical_downsampling
        );

        let mut new_cb = PixelMatrix::<u8>::new(downsampled_width, downsampled_height);
        let mut new_cr = PixelMatrix::<u8>::new(downsampled_width, downsampled_height);

        let add_average = |new_channel: &mut PixelMatrix<u8>, block_buffer: &mut Vec<u8>| {
            new_channel.push_next(
                (block_buffer
                    .iter()
                    .map(|x| *x as usize)
                    .sum::<usize>() / block_buffer.len()) as u8
            );
        };

        let mut add_average_cb = |block_buffer: &mut Vec<u8>| {
            add_average(&mut new_cb, block_buffer);
        };

        let mut add_average_cr = |block_buffer: &mut Vec<u8>| {
            add_average(&mut new_cr, block_buffer);
        };

        thread::scope(|s| {
            let cb_handle = s.spawn(|| {
                self.cb_channel
                    .get_block_iterator(horizontal_downsampling, vertical_downsampling, false)
                    .for_each_block(&mut add_average_cb);
            });

            let cr_handle = s.spawn(|| {
                self.cr_channel
                    .get_block_iterator(horizontal_downsampling, vertical_downsampling, false)
                    .for_each_block(&mut add_average_cr);
            });

            _ = cb_handle.join();
            _ = cr_handle.join();
        });

        self.cb_channel = new_cb;
        self.cr_channel = new_cr;
    }
}
