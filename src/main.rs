//! Creates a new BAM file.
//!
//! This writes a SAM header, reference sequences, and one unmapped record to stdout.
//!
//! Verify the output by piping to `samtools view --no-PG --with-header`.

use std::fs::File;
use std::io;
use std::io::{BufRead, Error, Stdout, Write};
use std::path::Path;
use noodles_bgzf as bgzf;
use noodles_sam::{self as sam, header::{Program, ReferenceSequence, reference_sequence}, Header};
use clap::{self, Parser, arg_enum};
use noodles_sam::header::header::{GroupOrder, SortOrder, Version};


struct PE {}
struct F {}
struct R {}

arg_enum!{
    #[derive(Debug, Clone)]
    pub enum Mode {
        PE,
        F,
        R,
    }
}


/// Convert 10x style fragments file to SAM stream
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Fragments file
    #[clap(short, long, value_parser)]
    fragments: String,

    /// Chromosome sizes file
    #[clap(short, long, value_parser)]
    genome: String,

    /// Output mode: PE, F, R
    #[clap(short, long, value_parser)]
    mode: Mode,
}


trait ParseAndSend {
    fn parse_and_send(line: & mut String, writer: & mut io::BufWriter<Stdout>, header: &Header);
}

// this should be a macro
impl ParseAndSend for PE {
    fn parse_and_send(line: & mut String, writer: & mut io::BufWriter<Stdout>, header: &Header) {
        let mut l = line.split("\t");

        let chr = l.next().expect("Couldn't read ref name");
        let start: usize = l.next().expect("Couldn't read start position")
            .parse()
            .expect("Couldn't parse start position")
            ;
        let end: usize = l.next()
            .expect("Couldn't read end position")
            .parse()
            .expect("Couldn't parse end position")
            ;
        let length: usize = end - start;
        let rn = l.next().expect("couldn't read the CB");

        if !header.reference_sequences().contains_key(chr) {
            return
        }

        writeln!(
            writer,
            "{}",
            format!("{}\t67\t{}\t{}\t255\t{}M\t=\t{}\t0\t*\t*", rn, chr, start+1, length, start+1)
        )
            .expect("Couldn't send line");
        writeln!(
            writer,
            "{}",
            format!("{}\t147\t{}\t{}\t255\t{}M\t=\t{}\t0\t*\t*", rn, chr, start+1, length, start+1)
        )
            .expect("Couldn't send line");
    }
}

impl ParseAndSend for F {
    fn parse_and_send(line: & mut String, writer: & mut io::BufWriter<Stdout>, header: &Header) {
        let mut l = line.split("\t");

        let chr = l.next().expect("Couldn't read ref name");
        let start: usize = l.next().expect("Couldn't read start position")
            .parse()
            .expect("Couldn't parse start position")
            ;
        let end: usize = l.next()
            .expect("Couldn't read end position")
            .parse()
            .expect("Couldn't parse end position")
            ;
        let length: usize = end - start;
        let rn = l.next().expect("couldn't read the CB");

        if !header.reference_sequences().contains_key(chr) {
            return
        }

        writeln!(
            writer,
            "{}",
            format!("{}\t67\t{}\t{}\t255\t{}M\t=\t{}\t0\t*\t*", rn, chr, start+1, length, start+1)
        )
            .expect("Couldn't send line");
    }
}


impl ParseAndSend for R {
    fn parse_and_send(line: & mut String, writer: & mut io::BufWriter<Stdout>, header: &Header) {
        let mut l = line.split("\t");

        let chr = l.next().expect("Couldn't read ref name");
        let start: usize = l.next().expect("Couldn't read start position")
            .parse()
            .expect("Couldn't parse start position")
            ;
        let end: usize = l.next()
            .expect("Couldn't read end position")
            .parse()
            .expect("Couldn't parse end position")
            ;
        let length: usize = end - start;
        let rn = l.next().expect("couldn't read the CB");

        if !header.reference_sequences().contains_key(chr) {
            return
        }

        writeln!(
            writer,
            "{}",
            format!("{}\t147\t{}\t{}\t255\t{}M\t=\t{}\t0\t*\t*", rn, chr, start+1, length, start+1)
        )
            .expect("Couldn't send line");
    }
}

fn read_genome(genome: &Path) -> Vec<(reference_sequence::Name, usize)> {
    let f = std::fs::File::open(genome).expect("Couldn't open genome file");
    let r = std::io::BufReader::new(f);

    let mut res: Vec<(reference_sequence::Name, usize)> = Vec::new();
    // that's some ugly stuff
    for line in r.lines().into_iter().collect::<Result<Vec<String>, Error>>().expect("asd") {
        let mut l = line
            .split("\t")
            ;

        let name: reference_sequence::Name = l.next().expect("Couldn't read name").parse().expect("Couldn't parse name into string");
        let length: usize = l.next().expect("Couldn't read length").parse().expect("Couldn't parse length into usize");
        res.push((name, length));
    }
    res
}





fn main() -> Result<(), Box<dyn std::error::Error>> {
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

    let mut writer = sam::Writer::new(io::stdout());

    let  header = sam::Header::builder()
        // .set_version(Version::new(1, 0))
        // .set_sort_order(SortOrder::Unknown).set_group_order(GroupOrder::Query)
        .set_header(
            sam::header::header::Header::builder()
                .set_version(Version::new(1, 0))
                .set_sort_order(SortOrder::Unknown)
                .set_group_order(GroupOrder::Query)
                .build()
        )
        .set_reference_sequences(reference_sequences)
        .add_program(Program::new("fragments_to_sam"))
        .add_comment("SAM output made with noodles")
        .build();


    writer.write_header(&header)?;
    drop(writer);

    let reader = File::open(fragment_file)
        .map(bgzf::Reader::new)
        .expect("Couldn't open the reader");

    let lines = reader.lines();

    let mut writer= io::BufWriter::new(io::stdout());

    match args.mode {
        Mode::PE => {
            for line in lines {
                PE::parse_and_send(& mut line.expect("Didn't receive a line"), & mut writer, &header);
            }
        },
        Mode::F => {
            for line in lines {
                F::parse_and_send(& mut line.expect("Didn't receive a line"), & mut writer, &header);
            }
        },
        Mode::R => {
            for line in lines {
                R::parse_and_send(& mut line.expect("Didn't receive a line"), & mut writer, &header);
            }
        },
    }


    Ok(())
}