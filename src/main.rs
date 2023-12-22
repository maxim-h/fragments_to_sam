use noodles_bgzf as bgzf;
use noodles_core::Position;
use noodles_sam::{
    self as sam,
    alignment::Record,
    header::{reference_sequence, Header, Program, ReferenceSequence},
    AsyncWriter,
};
use std::io::{BufRead, Error};
use std::path::Path;
// use noodles_sam::header::header::{GroupOrder, Header, SortOrder, Version};
use clap::Parser;
use noodles_sam::header::header::{GroupOrder, SortOrder, Version};
use noodles_sam::record::{Cigar, Flags, ReadName};
use tokio::io::{self, AsyncBufReadExt};

/// Convert 10x style fragments file to SAM stream
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long, value_parser)]
    fragments: String,

    #[clap(short, long, value_parser)]
    genome: String,
}

fn read_genome(genome: &Path) -> Vec<(reference_sequence::Name, usize)> {
    let f = std::fs::File::open(genome).expect("Couldn't open genome file");
    let r = std::io::BufReader::new(f);

    let mut res: Vec<(reference_sequence::Name, usize)> = Vec::new();
    // that's some ugly stuff
    for line in r
        .lines()
        .into_iter()
        .collect::<Result<Vec<String>, Error>>()
        .expect("asd")
    {
        let mut l = line.split("\t");

        let name: reference_sequence::Name = l
            .next()
            .expect("Couldn't read name")
            .parse()
            .expect("Couldn't parse name into string");
        let length: usize = l
            .next()
            .expect("Couldn't read length")
            .parse()
            .expect("Couldn't parse length into usize");
        res.push((name, length));
    }
    res
}

async fn parse_and_send(
    line: &mut String,
    writer: &mut AsyncWriter<tokio::io::Stdout>,
    header: &Header,
) {
    let mut l = line.split("\t");

    let length: usize;
    let alignment_start: Position;
    let ref_id: usize;

    let mut record = Record::builder()
        .set_reference_sequence_id({
            ref_id = match header
                .reference_sequences()
                .get_index_of(l.next().expect("Couldn't read ref name"))
            {
                Some(i) => i,
                None => return,
            };
            ref_id
        })
        .set_alignment_start({
            let start: usize = l
                .next()
                .expect("Couldn't read start position")
                .parse()
                .expect("Couldn't parse start position");
            let end: usize = l
                .next()
                .expect("Couldn't read end position")
                .parse()
                .expect("Couldn't parse end position");
            length = end - start;
            alignment_start = Position::new(start + 1).expect("couldn't convert to position");
            alignment_start
        })
        .set_read_name(
            ReadName::try_new(l.next().expect("couldn't read the CB"))
                .expect("Couldn't create ReadName"),
        )
        // .set_mapping_quality(MappingQuality::new().expect("this is bullshit"))
        .set_cigar(
            Cigar::try_from(vec![sam::record::cigar::Op::new(
                sam::record::cigar::op::Kind::Match,
                length,
            )])
            .expect("Couldn't create Cigar"),
        )
        .set_mate_reference_sequence_id(ref_id)
        .set_mate_alignment_start(alignment_start)
        .set_flags(Flags::from(67))
        .build();
    writer
        .write_record(header, &record)
        .await
        .expect("Couldn't write record");
    record.flags_mut().remove(Flags::from(67));
    record.flags_mut().insert(Flags::from(147));
    writer
        .write_record(header, &record)
        .await
        .expect("Couldn't write record");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let fragment_file = Path::new(&args.fragments);
    let rss = read_genome(Path::new(&args.genome));

    let reference_sequences = rss
        .into_iter()
        .map(|(name, len): (reference_sequence::Name, usize)| {
            let sn = name.to_string();
            ReferenceSequence::new(name, len).map(|rs| (sn, rs))
        })
        .collect::<Result<_, _>>()?;

    let mut writer = sam::AsyncWriter::new(io::stdout());

    let header = sam::Header::builder()
        // .set_version(Version::new(1, 0))
        // .set_sort_order(SortOrder::Unknown).set_group_order(GroupOrder::Query)
        .set_header(
            sam::header::header::Header::builder()
                .set_version(Version::new(1, 0))
                .set_sort_order(SortOrder::Unknown)
                .set_group_order(GroupOrder::Query)
                .build(),
        )
        .set_reference_sequences(reference_sequences)
        .add_program(Program::new("bed_to_bam"))
        .add_comment("SAM output made with noodles")
        .build();

    writer.write_header(&header).await?;

    let reader = tokio::fs::File::open(fragment_file)
        .await
        .map(bgzf::AsyncReader::new)
        .expect("Couldn't open the reader");

    let mut lines = reader.lines();

    loop {
        match lines.next_line().await.expect("Didn't receive a line") {
            Some(mut line) => {
                parse_and_send(&mut line, &mut writer, &header).await;
            }
            None => break,
        }
    }

    Ok(())
}
