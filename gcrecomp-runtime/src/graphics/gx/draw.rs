/// Translates accumulated GX vertex data + state into wgpu draw calls.
use super::vertex::DrawCall;
use wgpu::util::DeviceExt;
use wgpu::*;

/// A prepared draw command ready for wgpu submission.
pub struct PreparedDraw {
    pub vertex_buffer: Buffer,
    pub vertex_count: u32,
    pub primitive_topology: PrimitiveTopology,
}

/// Convert a GX primitive type byte to wgpu PrimitiveTopology.
pub fn gx_primitive_to_topology(prim: u8) -> PrimitiveTopology {
    match prim {
        0x90 => PrimitiveTopology::TriangleList,
        0x98 => PrimitiveTopology::TriangleStrip,
        0xA8 => PrimitiveTopology::LineList,
        0xB0 => PrimitiveTopology::LineStrip,
        0xB8 => PrimitiveTopology::PointList,
        // Quads (0x80) and TriangleFan (0xA0) need conversion
        _ => PrimitiveTopology::TriangleList,
    }
}

/// Convert GX quads to triangles (quad ABCD → triangles ABC + ACD).
pub fn convert_quads_to_triangles(vertices: &[f32], stride: usize) -> Vec<f32> {
    let vert_count = vertices.len() / stride;
    let quad_count = vert_count / 4;
    let mut result = Vec::with_capacity(quad_count * 6 * stride);

    for q in 0..quad_count {
        let base = q * 4 * stride;
        let a = &vertices[base..base + stride];
        let b = &vertices[base + stride..base + 2 * stride];
        let c = &vertices[base + 2 * stride..base + 3 * stride];
        let d = &vertices[base + 3 * stride..base + 4 * stride];

        // Triangle 1: A, B, C
        result.extend_from_slice(a);
        result.extend_from_slice(b);
        result.extend_from_slice(c);
        // Triangle 2: A, C, D
        result.extend_from_slice(a);
        result.extend_from_slice(c);
        result.extend_from_slice(d);
    }

    result
}

/// Convert GX triangle fan to triangle list (fan with center V0: V0V1V2, V0V2V3, ...).
pub fn convert_fan_to_triangles(vertices: &[f32], stride: usize) -> Vec<f32> {
    let vert_count = vertices.len() / stride;
    if vert_count < 3 {
        return Vec::new();
    }
    let tri_count = vert_count - 2;
    let mut result = Vec::with_capacity(tri_count * 3 * stride);

    let center = &vertices[0..stride];
    for i in 0..tri_count {
        let v1 = &vertices[(i + 1) * stride..(i + 2) * stride];
        let v2 = &vertices[(i + 2) * stride..(i + 3) * stride];
        result.extend_from_slice(center);
        result.extend_from_slice(v1);
        result.extend_from_slice(v2);
    }

    result
}

/// Prepare a draw call by creating the wgpu vertex buffer and handling
/// primitive conversion.
pub fn prepare_draw_call(device: &Device, draw_call: &DrawCall) -> PreparedDraw {
    let prim = draw_call.primitive as u8;
    let stride = draw_call.stride as usize;

    let (vertices, topology, vert_count) = match prim {
        0x80 => {
            // Quads → triangles
            let converted = convert_quads_to_triangles(&draw_call.vertex_data, stride);
            let count = converted.len() / stride;
            (converted, PrimitiveTopology::TriangleList, count)
        }
        0xA0 => {
            // Triangle fan → triangle list
            let converted = convert_fan_to_triangles(&draw_call.vertex_data, stride);
            let count = converted.len() / stride;
            (converted, PrimitiveTopology::TriangleList, count)
        }
        _ => {
            let topology = gx_primitive_to_topology(prim);
            let count = draw_call.vertex_data.len() / stride;
            (draw_call.vertex_data.clone(), topology, count)
        }
    };

    let vertex_bytes: &[u8] = bytemuck::cast_slice(&vertices);
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("GX Vertex Buffer"),
        contents: vertex_bytes,
        usage: BufferUsages::VERTEX,
    });

    PreparedDraw {
        vertex_buffer,
        vertex_count: vert_count as u32,
        primitive_topology: topology,
    }
}
