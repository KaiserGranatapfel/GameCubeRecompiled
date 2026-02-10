/// GX matrix operations: loading position/texture/projection matrices.
/// Load a 3x4 position/normal matrix into one of the 10 matrix slots.
/// GX stores model-view matrices as 3x4 (row-major), we pad to 4x4 for GPU.
pub fn load_pos_mtx_imm(matrices: &mut [[f32; 16]; 10], slot: u8, data: &[f32; 12]) {
    if (slot as usize) >= 10 {
        log::warn!("GXLoadPosMtxImm: invalid slot {}", slot);
        return;
    }
    // Convert 3x4 row-major to 4x4 column-major for wgpu uniform upload
    let m = &mut matrices[slot as usize];
    // Row 0
    m[0] = data[0];
    m[1] = data[4];
    m[2] = data[8];
    m[3] = 0.0;
    // Row 1
    m[4] = data[1];
    m[5] = data[5];
    m[6] = data[9];
    m[7] = 0.0;
    // Row 2
    m[8] = data[2];
    m[9] = data[6];
    m[10] = data[10];
    m[11] = 0.0;
    // Row 3 (translation)
    m[12] = data[3];
    m[13] = data[7];
    m[14] = data[11];
    m[15] = 1.0;
}

/// Load a 4x4 projection matrix. GX projection is either perspective or orthographic.
/// `proj_type`: 0 = perspective, 1 = orthographic.
pub fn load_projection_mtx(dest: &mut [f32; 16], data: &[f32], proj_type: u8) {
    // GX projection matrix is stored as 6 floats for perspective or 7 for ortho
    // We convert to standard 4x4 column-major
    *dest = [0.0; 16];

    if proj_type == 0 {
        // Perspective: data = [a, b, c, d, e, f]
        // Equivalent to:
        //   a  0  c  0
        //   0  b  d  0
        //   0  0  e  f
        //   0  0 -1  0
        if data.len() >= 6 {
            dest[0] = data[0]; // col 0, row 0
            dest[5] = data[1]; // col 1, row 1
            dest[8] = data[2]; // col 2, row 0
            dest[9] = data[3]; // col 2, row 1
            dest[10] = data[4]; // col 2, row 2
            dest[14] = data[5]; // col 3, row 2
            dest[11] = -1.0; // col 2, row 3
        }
    } else {
        // Orthographic: data = [a, b, c, d, e, f]
        //   a  0  0  d
        //   0  b  0  e
        //   0  0  c  f
        //   0  0  0  1
        if data.len() >= 6 {
            dest[0] = data[0];
            dest[5] = data[1];
            dest[10] = data[2];
            dest[12] = data[3];
            dest[13] = data[4];
            dest[14] = data[5];
            dest[15] = 1.0;
        }
    }
}

/// Load a 2x4 texture matrix into one of the 10 texture matrix slots.
pub fn load_tex_mtx_imm(matrices: &mut [[f32; 16]; 10], slot: u8, data: &[f32]) {
    if (slot as usize) >= 10 {
        log::warn!("GXLoadTexMtxImm: invalid slot {}", slot);
        return;
    }
    let m = &mut matrices[slot as usize];
    *m = [0.0; 16];
    // Copy available data (may be 2x4 = 8 floats or 3x4 = 12 floats)
    let count = data.len().min(12);
    for (i, &val) in data[..count].iter().enumerate() {
        let row = i / 4;
        let col = i % 4;
        // Store as column-major 4x4
        m[col * 4 + row] = val;
    }
    m[15] = 1.0;
}

/// Create an identity 4x4 matrix.
pub fn identity() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}
