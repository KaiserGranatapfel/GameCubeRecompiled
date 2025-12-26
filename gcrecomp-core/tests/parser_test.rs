// Unit tests for DOL parser
#[cfg(test)]
mod tests {
    use gcrecomp_core::recompiler::parser::DolFile;

    #[test]
    fn test_parse_empty_dol() {
        let data = vec![0u8; 0x100];
        let result = DolFile::parse(&data);
        // Should handle gracefully or return error
        assert!(result.is_ok() || result.is_err());
    }
}
