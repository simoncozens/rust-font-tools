use serde::{Deserialize, Serialize};

extern crate otspec;

use otspec::types::*;
use otspec_macros::tables;

tables!(maxp {
    Version16Dot16 version
    uint16  numGlyphs
    uint16  maxPoints
    uint16  maxContours
    uint16  maxCompositePoints
    uint16  maxCompositeContours
    uint16  maxZones
    uint16  maxTwilightPoints
    uint16  maxStorage
    uint16  maxFunctionDefs
    uint16  maxInstructionDefs
    uint16  maxStackElements
    uint16  maxSizeOfInstructions
    uint16  maxComponentElements
    uint16  maxComponentDepth
});
