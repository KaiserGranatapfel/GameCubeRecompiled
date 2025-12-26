// Unit tests for PowerPC decoder
#[cfg(test)]
mod tests {
    use gcrecomp_core::recompiler::decoder::Instruction;

    #[test]
    fn test_decode_addi() {
        // addi r3, r4, 5
        // Opcode 14, RT=3, RA=4, SI=5
        let word = (14u32 << 26) | (3u32 << 21) | (4u32 << 16) | 5u32;
        let result = Instruction::decode(word);
        assert!(result.is_ok());
    }
}

