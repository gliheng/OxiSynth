use super::super::utils::Reader;
use riff::Chunk;

#[derive(Default, Debug, Clone)]
pub struct SFSampleHeader {
    pub name: String,

    pub start: u32,
    pub end: u32,
    pub loop_start: u32,
    pub loop_end: u32,
    pub sample_rate: u32,

    pub origpitch: u8,
    pub pitchadj: i8,
    pub sample_link: u16,
    pub sample_type: u16,
}

impl SFSampleHeader {
    pub fn read(reader: &mut Reader) -> Self {
        let name: String = reader.read_string(20);
        // 20

        let start: u32 = reader.read_u32();
        // 24
        let end: u32 = reader.read_u32();
        // 28
        let loop_start: u32 = reader.read_u32();
        // 32
        let loop_end: u32 = reader.read_u32();
        // 36

        let sample_rate: u32 = reader.read_u32();
        // 40

        let origpitch: u8 = reader.read_u8();
        // 41
        let pitchadj: i8 = reader.read_i8();
        // 42
        let sample_link: u16 = reader.read_u16();
        // 44
        let sample_type: u16 = reader.read_u16();

        Self {
            name,
            start,
            end,
            loop_start,
            loop_end,
            sample_rate,
            origpitch,
            pitchadj,
            sample_link,
            sample_type,
        }
    }

    pub fn read_all(phdr: &Chunk, file: &mut std::fs::File) -> Vec<Self> {
        assert_eq!(phdr.id().as_str(), "shdr");

        let size = phdr.len();
        if size % 46 != 0 || size == 0 {
            panic!("Instrument header chunk size is invalid");
        }

        let amount = size / 46;

        let data = phdr.read_contents(file).unwrap();
        let mut reader = Reader::new(data);

        (0..amount).map(|_| Self::read(&mut reader)).collect()
    }
}