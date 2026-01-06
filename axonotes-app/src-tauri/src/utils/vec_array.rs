#[allow(dead_code)]
pub trait ByteArrayConversion {
    /// Convert Vec<u8> to fixed-size array [u8; N]
    fn to_array<const N: usize>(self) -> Result<[u8; N], String>;

    /// Convert Vec<u8> to fixed-size array reference &[u8; N]
    fn as_array<const N: usize>(&self) -> Result<&[u8; N], String>;
}

impl ByteArrayConversion for Vec<u8> {
    fn to_array<const N: usize>(self) -> Result<[u8; N], String> {
        self.try_into()
            .map_err(|v: Vec<u8>| format!("Expected {} bytes, got {}", N, v.len()))
    }

    fn as_array<const N: usize>(&self) -> Result<&[u8; N], String> {
        self.as_slice()
            .try_into()
            .map_err(|_| format!("Expected {} bytes, got {}", N, self.len()))
    }
}
