// TEV (Texture Environment) stage configuration for GameCube GX pipeline.
//
// The GameCube GPU has 16 TEV stages that combine textures, rasterized
// colors, and constant colors to produce final pixel output. Each stage
// computes: d + (1 - c) * a + c * b, with configurable bias, scale, and
// clamping. This module stores per-stage configuration and generates
// dynamic WGSL fragment shader code for the active TEV stages.

use std::fmt::Write;

// ---------------------------------------------------------------------------
// TEV enums
// ---------------------------------------------------------------------------

/// Color channel input selector for a TEV stage.
///
/// Each TEV stage has four color inputs (a, b, c, d). This enum selects
/// which source feeds into each slot. Values match the hardware register
/// encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TevColorArg {
    /// Previous stage color RGB.
    CprevRgb = 0,
    /// Previous stage alpha broadcast to RGB.
    AprevRgb = 1,
    /// Color register 0 RGB.
    C0Rgb = 2,
    /// Alpha register 0 broadcast to RGB.
    A0Rgb = 3,
    /// Color register 1 RGB.
    C1Rgb = 4,
    /// Alpha register 1 broadcast to RGB.
    A1Rgb = 5,
    /// Color register 2 RGB.
    C2Rgb = 6,
    /// Alpha register 2 broadcast to RGB.
    A2Rgb = 7,
    /// Texture color RGB.
    TexcRgb = 8,
    /// Texture alpha broadcast to RGB.
    TexaRgb = 9,
    /// Rasterized color RGB.
    RascRgb = 10,
    /// Constant one (vec3(1.0)).
    One = 11,
    /// Constant half (vec3(0.5)).
    Half = 12,
    /// Konst color selection (per-stage configurable constant).
    Konst = 13,
    /// Constant zero (vec3(0.0)).
    Zero = 14,
}

/// Alpha channel input selector for a TEV stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TevAlphaArg {
    /// Previous stage alpha.
    AprevAlpha = 0,
    /// Alpha register 0.
    A0Alpha = 1,
    /// Alpha register 1.
    A1Alpha = 2,
    /// Alpha register 2.
    A2Alpha = 3,
    /// Texture alpha.
    TexAlpha = 4,
    /// Rasterized alpha.
    RasAlpha = 5,
    /// Konst alpha selection.
    KonstAlpha = 6,
    /// Constant zero.
    Zero = 7,
}

/// Arithmetic operation applied in a TEV stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TevOp {
    Add = 0,
    Sub = 1,
}

/// Output scale factor applied after the TEV combine operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TevScale {
    Scale1 = 0,
    Scale2 = 1,
    Scale4 = 2,
    DivideBy2 = 3,
}

/// Destination register for a TEV stage output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TevRegId {
    /// The implicit "previous" register passed between stages.
    Prev = 0,
    Reg0 = 1,
    Reg1 = 2,
    Reg2 = 3,
}

// ---------------------------------------------------------------------------
// TEV stage configuration
// ---------------------------------------------------------------------------

/// Complete configuration for a single TEV stage.
///
/// Each stage computes separate color and alpha results using the formula:
///   result = d OP ((1 - c) * a + c * b) + bias
/// The result is then scaled and optionally clamped before being written
/// to the destination register.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TevStageConfig {
    /// Color channel inputs [a, b, c, d].
    pub color_in: [TevColorArg; 4],
    /// Alpha channel inputs [a, b, c, d].
    pub alpha_in: [TevAlphaArg; 4],

    /// Color combine operation.
    pub color_op: TevOp,
    /// Alpha combine operation.
    pub alpha_op: TevOp,

    /// Color bias selector (hardware encoding: 0=zero, 1=+0.5, 2=-0.5).
    pub color_bias: u8,
    /// Alpha bias selector.
    pub alpha_bias: u8,

    /// Whether to clamp color output to [0, 1].
    pub color_clamp: bool,
    /// Whether to clamp alpha output to [0, 1].
    pub alpha_clamp: bool,

    /// Color output scale.
    pub color_scale: TevScale,
    /// Alpha output scale.
    pub alpha_scale: TevScale,

    /// Destination register for the color result.
    pub color_dest: TevRegId,
    /// Destination register for the alpha result.
    pub alpha_dest: TevRegId,

    /// Texture coordinate index used for texture lookup.
    pub tex_coord: u8,
    /// Texture map index used for texture lookup.
    pub tex_map: u8,
    /// Color channel index (selects which rasterized color to use).
    pub channel: u8,

    /// Konst color selector (hardware register value).
    pub konst_color_sel: u8,
    /// Konst alpha selector (hardware register value).
    pub konst_alpha_sel: u8,
}

impl Default for TevStageConfig {
    /// Returns a default pass-through TEV stage configuration.
    ///
    /// Color: d = CprevRgb with a/b/c = Zero, so the output is simply
    /// the previous stage color. Same for alpha with AprevAlpha.
    /// Operation is Add with scale 1, no bias, clamped, writing to Prev.
    fn default() -> Self {
        Self {
            color_in: [
                TevColorArg::Zero,
                TevColorArg::Zero,
                TevColorArg::Zero,
                TevColorArg::CprevRgb,
            ],
            alpha_in: [
                TevAlphaArg::Zero,
                TevAlphaArg::Zero,
                TevAlphaArg::Zero,
                TevAlphaArg::AprevAlpha,
            ],
            color_op: TevOp::Add,
            alpha_op: TevOp::Add,
            color_bias: 0,
            alpha_bias: 0,
            color_clamp: true,
            alpha_clamp: true,
            color_scale: TevScale::Scale1,
            alpha_scale: TevScale::Scale1,
            color_dest: TevRegId::Prev,
            alpha_dest: TevRegId::Prev,
            tex_coord: 0,
            tex_map: 0,
            channel: 0,
            konst_color_sel: 0,
            konst_alpha_sel: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// WGSL code generation helpers
// ---------------------------------------------------------------------------

/// Maps a `TevColorArg` to its WGSL vec3<f32> expression string.
fn color_arg_to_wgsl(arg: TevColorArg) -> &'static str {
    match arg {
        TevColorArg::CprevRgb => "tev_prev.rgb",
        TevColorArg::AprevRgb => "vec3<f32>(tev_prev.a)",
        TevColorArg::C0Rgb => "tev_reg0.rgb",
        TevColorArg::A0Rgb => "vec3<f32>(tev_reg0.a)",
        TevColorArg::C1Rgb => "tev_reg1.rgb",
        TevColorArg::A1Rgb => "vec3<f32>(tev_reg1.a)",
        TevColorArg::C2Rgb => "tev_reg2.rgb",
        TevColorArg::A2Rgb => "vec3<f32>(tev_reg2.a)",
        TevColorArg::TexcRgb => "tex_color.rgb",
        TevColorArg::TexaRgb => "vec3<f32>(tex_color.a)",
        TevColorArg::RascRgb => "ras_color.rgb",
        TevColorArg::One => "vec3<f32>(1.0)",
        TevColorArg::Half => "vec3<f32>(0.5)",
        TevColorArg::Konst => "konst_color.rgb",
        TevColorArg::Zero => "vec3<f32>(0.0)",
    }
}

/// Maps a `TevAlphaArg` to its WGSL f32 expression string.
fn alpha_arg_to_wgsl(arg: TevAlphaArg) -> &'static str {
    match arg {
        TevAlphaArg::AprevAlpha => "tev_prev.a",
        TevAlphaArg::A0Alpha => "tev_reg0.a",
        TevAlphaArg::A1Alpha => "tev_reg1.a",
        TevAlphaArg::A2Alpha => "tev_reg2.a",
        TevAlphaArg::TexAlpha => "tex_color.a",
        TevAlphaArg::RasAlpha => "ras_color.a",
        TevAlphaArg::KonstAlpha => "konst_color.a",
        TevAlphaArg::Zero => "0.0",
    }
}

/// Maps a `TevOp` to its WGSL arithmetic symbol.
fn op_to_wgsl(op: TevOp) -> &'static str {
    match op {
        TevOp::Add => "+",
        TevOp::Sub => "-",
    }
}

/// Maps a `TevScale` to its WGSL multiplier literal.
fn scale_to_wgsl(scale: TevScale) -> &'static str {
    match scale {
        TevScale::Scale1 => "1.0",
        TevScale::Scale2 => "2.0",
        TevScale::Scale4 => "4.0",
        TevScale::DivideBy2 => "0.5",
    }
}

/// Maps a bias selector byte to its WGSL addend literal.
fn bias_to_wgsl(bias: u8) -> &'static str {
    match bias {
        0 => "0.0",
        1 => "0.5",
        2 => "-0.5",
        _ => "0.0",
    }
}

/// Maps a `TevRegId` to its WGSL variable name.
fn reg_to_wgsl(reg: TevRegId) -> &'static str {
    match reg {
        TevRegId::Prev => "tev_prev",
        TevRegId::Reg0 => "tev_reg0",
        TevRegId::Reg1 => "tev_reg1",
        TevRegId::Reg2 => "tev_reg2",
    }
}

// ---------------------------------------------------------------------------
// WGSL generation
// ---------------------------------------------------------------------------

/// Generates WGSL fragment shader code for the given TEV stage pipeline.
///
/// The returned string is a self-contained WGSL fragment function body
/// (without the `@fragment fn` wrapper) that declares TEV registers,
/// iterates over the active stages, and writes the final color to
/// `tev_prev`. The caller is responsible for embedding this into a
/// complete shader that provides `tex_color`, `ras_color`, and
/// `konst_color` bindings.
///
/// # Arguments
///
/// * `stages` - Slice of TEV stage configurations (up to 16).
/// * `num_stages` - Number of active stages to generate code for.
///
/// # Returns
///
/// A `String` containing the WGSL code for all active TEV stages.
pub fn generate_tev_wgsl(stages: &[TevStageConfig], num_stages: u8) -> String {
    let count = (num_stages as usize).min(stages.len()).min(16);
    let mut out = String::with_capacity(2048);

    // Declare TEV registers.
    writeln!(out, "    // TEV registers").unwrap();
    writeln!(out, "    var tev_prev: vec4<f32> = vec4<f32>(0.0);").unwrap();
    writeln!(out, "    var tev_reg0: vec4<f32> = vec4<f32>(0.0);").unwrap();
    writeln!(out, "    var tev_reg1: vec4<f32> = vec4<f32>(0.0);").unwrap();
    writeln!(out, "    var tev_reg2: vec4<f32> = vec4<f32>(0.0);").unwrap();
    writeln!(out).unwrap();

    for (i, stage) in stages[..count].iter().enumerate() {
        generate_stage_wgsl(&mut out, stage, i);
    }

    out
}

/// Appends the WGSL code for a single TEV stage to `out`.
fn generate_stage_wgsl(out: &mut String, stage: &TevStageConfig, index: usize) {
    let n = index;

    writeln!(out, "    // TEV Stage {n}").unwrap();

    // Color inputs.
    let ca = color_arg_to_wgsl(stage.color_in[0]);
    let cb = color_arg_to_wgsl(stage.color_in[1]);
    let cc = color_arg_to_wgsl(stage.color_in[2]);
    let cd = color_arg_to_wgsl(stage.color_in[3]);

    writeln!(out, "    let ca_{n} = {ca};").unwrap();
    writeln!(out, "    let cb_{n} = {cb};").unwrap();
    writeln!(out, "    let cc_{n} = {cc};").unwrap();
    writeln!(out, "    let cd_{n} = {cd};").unwrap();

    // Alpha inputs.
    let aa = alpha_arg_to_wgsl(stage.alpha_in[0]);
    let ab = alpha_arg_to_wgsl(stage.alpha_in[1]);
    let ac = alpha_arg_to_wgsl(stage.alpha_in[2]);
    let ad = alpha_arg_to_wgsl(stage.alpha_in[3]);

    writeln!(out, "    let aa_{n} = {aa};").unwrap();
    writeln!(out, "    let ab_{n} = {ab};").unwrap();
    writeln!(out, "    let ac_{n} = {ac};").unwrap();
    writeln!(out, "    let ad_{n} = {ad};").unwrap();

    // Color combine: d OP ((1 - c) * a + c * b) + bias, then scale.
    let cop = op_to_wgsl(stage.color_op);
    let cbias = bias_to_wgsl(stage.color_bias);
    let cscale = scale_to_wgsl(stage.color_scale);

    writeln!(
        out,
        "    let color_{n} = (cd_{n} {cop} \
         ((vec3<f32>(1.0) - cc_{n}) * ca_{n} + cc_{n} * cb_{n}) \
         + vec3<f32>({cbias})) * {cscale};"
    )
    .unwrap();

    // Alpha combine.
    let aop = op_to_wgsl(stage.alpha_op);
    let abias = bias_to_wgsl(stage.alpha_bias);
    let ascale = scale_to_wgsl(stage.alpha_scale);

    writeln!(
        out,
        "    let alpha_{n} = (ad_{n} {aop} \
         ((1.0 - ac_{n}) * aa_{n} + ac_{n} * ab_{n}) \
         + {abias}) * {ascale};"
    )
    .unwrap();

    // Clamping.
    let color_expr = if stage.color_clamp {
        format!("clamp(color_{n}, vec3<f32>(0.0), vec3<f32>(1.0))")
    } else {
        format!("color_{n}")
    };

    let alpha_expr = if stage.alpha_clamp {
        format!("clamp(alpha_{n}, 0.0, 1.0)")
    } else {
        format!("alpha_{n}")
    };

    // Write to destination register.
    let cdest = reg_to_wgsl(stage.color_dest);
    let adest = reg_to_wgsl(stage.alpha_dest);

    if stage.color_dest == stage.alpha_dest {
        // Both channels write to the same register -- emit one
        // combined vec4 assignment.
        writeln!(out, "    {cdest} = vec4<f32>({color_expr}, {alpha_expr});").unwrap();
    } else {
        writeln!(out, "    {cdest} = vec4<f32>({color_expr}, {cdest}.a);").unwrap();
        writeln!(out, "    {adest} = vec4<f32>({adest}.rgb, {alpha_expr});").unwrap();
    }

    writeln!(out).unwrap();
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_stage_is_passthrough() {
        let stage = TevStageConfig::default();
        assert_eq!(stage.color_in[3], TevColorArg::CprevRgb);
        assert_eq!(stage.alpha_in[3], TevAlphaArg::AprevAlpha);
        assert_eq!(stage.color_op, TevOp::Add);
        assert_eq!(stage.alpha_op, TevOp::Add);
        assert_eq!(stage.color_scale, TevScale::Scale1);
        assert_eq!(stage.alpha_scale, TevScale::Scale1);
        assert_eq!(stage.color_dest, TevRegId::Prev);
        assert_eq!(stage.alpha_dest, TevRegId::Prev);
        assert!(stage.color_clamp);
        assert!(stage.alpha_clamp);
    }

    #[test]
    fn default_passthrough_wgsl_contains_prev() {
        let stages = [TevStageConfig::default()];
        let wgsl = generate_tev_wgsl(&stages, 1);

        assert!(wgsl.contains("// TEV Stage 0"));
        assert!(wgsl.contains("tev_prev.rgb"));
        assert!(wgsl.contains("tev_prev.a"));
        assert!(wgsl.contains("tev_prev = vec4<f32>"));
    }

    #[test]
    fn zero_stages_produces_only_register_decls() {
        let wgsl = generate_tev_wgsl(&[], 0);
        assert!(wgsl.contains("var tev_prev"));
        assert!(!wgsl.contains("// TEV Stage"));
    }

    #[test]
    fn num_stages_clamped_to_slice_length() {
        let stages = [TevStageConfig::default()];
        // Request 4 stages but only 1 exists in the slice.
        let wgsl = generate_tev_wgsl(&stages, 4);
        assert!(wgsl.contains("// TEV Stage 0"));
        assert!(!wgsl.contains("// TEV Stage 1"));
    }

    #[test]
    fn num_stages_clamped_to_16() {
        let stages = [TevStageConfig::default(); 20];
        let wgsl = generate_tev_wgsl(&stages, 20);
        assert!(wgsl.contains("// TEV Stage 15"));
        assert!(!wgsl.contains("// TEV Stage 16"));
    }

    #[test]
    fn texture_inputs_produce_tex_color() {
        let mut stage = TevStageConfig::default();
        stage.color_in[0] = TevColorArg::TexcRgb;
        stage.alpha_in[0] = TevAlphaArg::TexAlpha;

        let stages = [stage];
        let wgsl = generate_tev_wgsl(&stages, 1);

        assert!(wgsl.contains("tex_color.rgb"));
        assert!(wgsl.contains("tex_color.a"));
    }

    #[test]
    fn sub_op_produces_minus() {
        let stage = TevStageConfig {
            color_op: TevOp::Sub,
            alpha_op: TevOp::Sub,
            ..Default::default()
        };

        let stages = [stage];
        let wgsl = generate_tev_wgsl(&stages, 1);

        assert!(wgsl.contains("cd_0 -"));
        assert!(wgsl.contains("ad_0 -"));
    }

    #[test]
    fn scale2_appears_in_output() {
        let stage = TevStageConfig {
            color_scale: TevScale::Scale2,
            ..Default::default()
        };

        let stages = [stage];
        let wgsl = generate_tev_wgsl(&stages, 1);

        assert!(wgsl.contains("* 2.0;"));
    }

    #[test]
    fn separate_color_alpha_dest_registers() {
        let stage = TevStageConfig {
            color_dest: TevRegId::Reg0,
            alpha_dest: TevRegId::Reg1,
            ..Default::default()
        };

        let stages = [stage];
        let wgsl = generate_tev_wgsl(&stages, 1);

        assert!(wgsl.contains("tev_reg0 = vec4<f32>("));
        assert!(wgsl.contains("tev_reg1 = vec4<f32>("));
    }

    #[test]
    fn bias_half_appears_in_output() {
        let stage = TevStageConfig {
            color_bias: 1, // +0.5
            ..Default::default()
        };

        let stages = [stage];
        let wgsl = generate_tev_wgsl(&stages, 1);

        assert!(wgsl.contains("vec3<f32>(0.5)"));
    }

    #[test]
    fn no_clamp_omits_clamp_call() {
        let stage = TevStageConfig {
            color_clamp: false,
            alpha_clamp: false,
            ..Default::default()
        };

        let stages = [stage];
        let wgsl = generate_tev_wgsl(&stages, 1);

        // When clamping is disabled the raw expression is used directly.
        assert!(wgsl.contains("tev_prev = vec4<f32>(color_0, alpha_0)"));
    }

    #[test]
    fn multi_stage_generates_sequential_blocks() {
        let stages = [TevStageConfig::default(); 3];
        let wgsl = generate_tev_wgsl(&stages, 3);

        assert!(wgsl.contains("// TEV Stage 0"));
        assert!(wgsl.contains("// TEV Stage 1"));
        assert!(wgsl.contains("// TEV Stage 2"));
        assert!(wgsl.contains("ca_1"));
        assert!(wgsl.contains("alpha_2"));
    }

    #[test]
    fn all_color_args_produce_valid_wgsl() {
        // Smoke test: every TevColorArg variant produces a non-empty
        // string that does not contain "UNKNOWN".
        let all = [
            TevColorArg::CprevRgb,
            TevColorArg::AprevRgb,
            TevColorArg::C0Rgb,
            TevColorArg::A0Rgb,
            TevColorArg::C1Rgb,
            TevColorArg::A1Rgb,
            TevColorArg::C2Rgb,
            TevColorArg::A2Rgb,
            TevColorArg::TexcRgb,
            TevColorArg::TexaRgb,
            TevColorArg::RascRgb,
            TevColorArg::One,
            TevColorArg::Half,
            TevColorArg::Konst,
            TevColorArg::Zero,
        ];
        for arg in all {
            let s = color_arg_to_wgsl(arg);
            assert!(!s.is_empty(), "{arg:?} produced empty WGSL");
        }
    }

    #[test]
    fn all_alpha_args_produce_valid_wgsl() {
        let all = [
            TevAlphaArg::AprevAlpha,
            TevAlphaArg::A0Alpha,
            TevAlphaArg::A1Alpha,
            TevAlphaArg::A2Alpha,
            TevAlphaArg::TexAlpha,
            TevAlphaArg::RasAlpha,
            TevAlphaArg::KonstAlpha,
            TevAlphaArg::Zero,
        ];
        for arg in all {
            let s = alpha_arg_to_wgsl(arg);
            assert!(!s.is_empty(), "{arg:?} produced empty WGSL");
        }
    }

    #[test]
    fn konst_color_appears_in_output() {
        let mut stage = TevStageConfig::default();
        stage.color_in[0] = TevColorArg::Konst;
        stage.alpha_in[0] = TevAlphaArg::KonstAlpha;

        let stages = [stage];
        let wgsl = generate_tev_wgsl(&stages, 1);

        assert!(wgsl.contains("konst_color.rgb"));
        assert!(wgsl.contains("konst_color.a"));
    }
}
