/*
 * GameCube Linker Script Template
 * 
 * This linker script defines memory layout and namespace organization
 * for GameCube recompiled code. Customize this template for your specific game.
 */

/* Memory regions for GameCube */
MEMORY {
    TEXT : ORIGIN = 0x80003100, LENGTH = 0x1000000
    DATA : ORIGIN = 0x80400000, LENGTH = 0x1000000
    BSS  : ORIGIN = 0x80500000, LENGTH = 0x1000000
}

/* Section definitions */
SECTIONS {
    .text : {
        *(.text.*)
    } > TEXT
    
    .data : {
        *(.data.*)
    } > DATA
    
    .bss : {
        *(.bss.*)
    } > BSS
}

/* Namespace organization rules */
/* Functions matching patterns will be organized into modules */
NAMESPACES {
    /* Graphics subsystem */
    "gx" : {
        pattern: "GX*",
        module: "graphics/gx",
        namespace: ["graphics"]
    }
    
    "vi" : {
        pattern: "VI*",
        module: "graphics/vi",
        namespace: ["graphics"]
    }
    
    /* Audio subsystem */
    "dsp" : {
        pattern: "DSP*",
        module: "audio/dsp",
        namespace: ["audio"]
    }
    
    "ai" : {
        pattern: "AI*",
        module: "audio/ai",
        namespace: ["audio"]
    }
    
    /* System subsystem */
    "os" : {
        pattern: "OS*",
        module: "system/os",
        namespace: ["system"]
    }
    
    "exi" : {
        pattern: "EXI*",
        module: "system/exi",
        namespace: ["system"]
    }
    
    "si" : {
        pattern: "SI*",
        module: "system/si",
        namespace: ["system"]
    }
    
    "dvd" : {
        pattern: "DVD*",
        module: "system/dvd",
        namespace: ["system"]
    }
    
    "card" : {
        pattern: "CARD*",
        module: "system/card",
        namespace: ["system"]
    }
    
    /* Memory subsystem */
    "mem" : {
        pattern: "MEM*",
        module: "system/mem",
        namespace: ["system"]
    }
    
    /* Math library */
    "mtx" : {
        pattern: "MTX*",
        module: "math/mtx",
        namespace: ["math"]
    }
    
    "vec" : {
        pattern: "VEC*",
        module: "math/vec",
        namespace: ["math"]
    }
    
    /* Unknown functions go to unknown module */
    "unknown" : {
        pattern: "*",
        module: "unknown",
        namespace: []
    }
}

