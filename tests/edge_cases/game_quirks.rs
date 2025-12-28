//! Tests for game-specific quirk handling

use gcrecomp_core::recompiler::game_quirks::{GameQuirkDatabase, QuirkPattern, QuirkType, Workaround};

#[test]
fn test_quirk_database() {
    let db = GameQuirkDatabase::new();
    
    // Test finding quirks for a game
    let quirks = db.find_quirks("TEST_GAME");
    assert_eq!(quirks.len(), 0); // No quirks for test game
    
    // Test address pattern matching
    let result = db.check_address("TEST_GAME", 0x80000000);
    assert!(result.is_none());
}

#[test]
fn test_quirk_pattern_matching() {
    let db = GameQuirkDatabase::new();
    
    // Test instruction pattern matching
    let instructions = vec!["mtctr".to_string(), "bctrl".to_string()];
    let result = db.check_instructions("TEST_GAME", &instructions);
    assert!(result.is_none()); // No quirks defined
}

#[test]
fn test_quirk_workaround() {
    let mut db = GameQuirkDatabase::new();
    
    // Create a test quirk
    let quirk = QuirkPattern {
        pattern_type: QuirkType::JumpTable,
        description: "Test jump table".to_string(),
        address_pattern: Some("0x80000000:*".to_string()),
        instruction_pattern: None,
        workaround: Workaround::SkipOptimization,
    };
    
    // Apply workaround (should not error)
    let result = db.apply_workaround(&quirk);
    assert!(result.is_ok());
}

