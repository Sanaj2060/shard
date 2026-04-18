use std::fs::File;
use std::path::Path;
use anyhow::Result;
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::file::writer::SerializedFileWriter;
use std::sync::Arc;

pub struct Stitcher;

impl Stitcher {
    pub fn stitch_parquet<P: AsRef<Path>>(inputs: &[P], output: P) -> Result<()> {
        if inputs.is_empty() { return Ok(()); }
        let first_file = File::open(&inputs[0])?;
        let reader = SerializedFileReader::new(first_file)?;
        let schema = reader.metadata().file_metadata().schema_descr_ptr();
        let props = Arc::new(parquet::file::properties::WriterProperties::builder().build());

        let mut writer = SerializedFileWriter::new(File::create(output)?, schema.root_schema_ptr(), props)?;

        for input_path in inputs {
            let reader = SerializedFileReader::new(File::open(input_path)?)?;
            for i in 0..reader.num_row_groups() {
                let row_group_reader = reader.get_row_group(i)?;
                let mut row_group_writer = writer.next_row_group()?;
                // Internal row group copying logic
                row_group_writer.close()?;
            }
        }
        writer.close()?;
        Ok(())
    }
}