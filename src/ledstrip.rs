use std::io::{self, Write};

static SOF: u8 = 0x72;

pub fn fill_colour<T: Write>(spi_device: &mut T) -> io::Result<()> {
    let mut data = [0xFF; 256 * 3 + 1]; // 3 bytes per pixel + 1 for SOF.
    data[0] = SOF;
    spi_device.write(&data)?;
    Ok(())
}
