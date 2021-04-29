
struct PackedPoints {

}

deserialize_visitor!(
    PackedPoints,
    PackedPointsVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut count1 :u16 = read_field!(seq, u8, "a packed point count (first byte)") as u16;
        if count1 > 0 && count1 < 128 {
            let count2: u16 = read_field!(seq, u8, "a packed point count (second byte)") as u16;
        }
    }
)

