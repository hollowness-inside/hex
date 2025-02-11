//! general hex lib

#[cfg(test)]
mod tests;

mod format;
use crate::format::Format;

use ansi_term::Color;
use clap::ArgMatches;
use no_color::is_no_color;
use std::env;
use std::error::Error;
use std::f64;
use std::fs::File;
use std::io::BufReader;
use std::io::IsTerminal;
use std::io::{self, BufRead, Read, Write};

/// arg cols
pub const ARG_COL: &str = "cols";
/// arg len
pub const ARG_LEN: &str = "len";
/// arg format
pub const ARG_FMT: &str = "format";
/// arg INPUTFILE
pub const ARG_INP: &str = "INPUTFILE";
/// arg color
pub const ARG_CLR: &str = "color";
/// arg array
pub const ARG_ARR: &str = "array";
/// arg func
pub const ARG_FNC: &str = "func";
/// arg places
pub const ARG_PLC: &str = "places";
/// arg prefix
pub const ARG_PFX: &str = "prefix";

const ARGS: [&str; 9] = [
    ARG_COL, ARG_LEN, ARG_FMT, ARG_INP, ARG_CLR, ARG_ARR, ARG_FNC, ARG_PLC, ARG_PFX,
];

const DBG: bool = false;

/// Line structure for hex output
#[derive(Clone, Debug, Default)]
pub struct Line {
    /// offset
    pub offset: u64,
    /// hex body
    pub hex_body: Vec<u8>,
    /// ascii text
    pub ascii: Vec<u8>,
    /// total bytes in Line
    pub bytes: u64,
}

/// Line implementation
impl Line {
    /// Line constructor
    pub fn new() -> Line {
        Line {
            offset: 0x0,
            hex_body: Vec::new(),
            ascii: Vec::new(),
            bytes: 0x0,
        }
    }
}

/// Page structure
#[derive(Clone, Debug, Default)]
pub struct Page {
    /// page offset
    pub offset: u64,
    /// page body
    pub body: Vec<Line>,
    /// total bytes in page
    pub bytes: u64,
}

/// Page implementation
impl Page {
    /// Page constructor
    pub fn new() -> Page {
        Page {
            offset: 0x0,
            body: Vec::new(),
            bytes: 0x0,
        }
    }
}

/// offset column
///
/// # Arguments
///
/// * `b` - offset value.
pub fn offset(b: u64) -> String {
    format!("{b:#08x}")
}

/// print offset to std out
pub fn print_offset(w: &mut impl Write, b: u64) -> io::Result<()> {
    write!(w, "{}: ", offset(b))
}

/// print byte to std out
pub fn print_byte(
    w: &mut impl Write,
    b: u8,
    format: Format,
    colorize: bool,
    prefix: bool,
) -> io::Result<()> {
    let fmt_string = format.format(b, prefix);
    if colorize {
        // note, for color testing: for (( i = 0; i < 256; i++ )); do echo "$(tput setaf $i)This is ($i) $(tput sgr0)"; done
        let color = byte_to_color(b);
        let string = ansi_term::Style::new().fg(color).paint(fmt_string);
        write!(w, "{string} ")
    } else {
        write!(w, "{fmt_string} ")
    }
}

/// get the color for a specific byte
pub fn byte_to_color(b: u8) -> Color {
    let color = match b {
        0 => 0x16,
        _ => b,
    };

    ansi_term::Color::Fixed(color)
}

/// append char representation of a byte to a buffer
pub fn append_ascii(target: &mut Vec<u8>, b: u8, colorize: bool) {
    let chr = match b > 31 && b < 127 {
        true => b as char,
        false => '.',
    };

    if colorize {
        let string = ansi_term::Style::new()
            .fg(byte_to_color(b))
            .paint(chr.to_string());

        target.extend(format!("{string}").as_bytes());
    } else {
        target.extend(format!("{chr}").as_bytes());
    }
}

/// In most hex editor applications, the data of the computer file is
/// represented as hexadecimal values grouped in 4 groups of 4 bytes (or
/// two groups of 8 bytes), followed by one group of 16 printable ASCII
/// characters which correspond to each pair of hex values (each byte).
/// Non-printable ASCII characters (e.g., Bell) and characters that would take
/// more than one character space (e.g., tab) are typically represented by a
/// dot (".") in the following ASCII field.
///
/// # Arguments
///
/// * `matches` - Argument matches from command line.
pub fn run(matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let mut column_width: u64 = 10;
    let mut truncate_len: u64 = 0x0;
    if let Some(len) = matches.get_one::<String>("func") {
        let mut p: usize = 4;
        if let Some(places) = matches.get_one::<String>("places") {
            p = match places.parse::<usize>() {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("-p, --places <integer> expected. {:?}", e);
                    return Err(Box::new(e));
                }
            }
        }
        output_function(len.parse::<u64>().unwrap(), p);
    } else {
        // cases:
        //  $ cat Cargo.toml | target/debug/hx
        //  $ cat Cargo.toml | target/debug/hx -a r
        //  $ target/debug/hx Cargo.toml
        //  $ target/debug/hx Cargo.toml -a r
        let mut buf: Box<dyn BufRead> = match is_stdin(&matches) {
            true => Box::new(BufReader::new(io::stdin())),
            false => {
                let path = matches.get_one::<String>(ARG_INP).unwrap();
                let file = File::open(path)?;
                Box::new(BufReader::new(file))
            }
        };
        let mut format_out = Format::LowerHex;
        let mut colorize = true;
        let mut prefix = true;

        if let Some(columns) = matches.get_one::<String>(ARG_COL) {
            column_width = match columns.parse::<u64>() {
                Ok(column_width) => column_width,
                Err(e) => {
                    eprintln!("-c, --cols <integer> expected. {:?}", e);
                    return Err(Box::new(e));
                }
            }
        }

        if let Some(length) = matches.get_one::<String>(ARG_LEN) {
            truncate_len = match length.parse::<u64>() {
                Ok(truncate_len) => truncate_len,
                Err(e) => {
                    eprintln!("-l, --len <integer> expected. {:?}", e);
                    return Err(Box::new(e));
                }
            }
        }

        if let Some(format) = matches.get_one::<String>(ARG_FMT) {
            // o, x, X, p, b, e, E
            match format.as_str() {
                "o" => format_out = Format::Octal,
                "x" => format_out = Format::LowerHex,
                "X" => format_out = Format::UpperHex,
                "p" => format_out = Format::Pointer,
                "b" => format_out = Format::Binary,
                "e" => format_out = Format::LowerExp,
                "E" => format_out = Format::UpperExp,
                _ => format_out = Format::Unknown,
            }
        }

        // check no_color here
        // override via ARG_CLR below
        if is_no_color() {
            colorize = false;
        }

        // prevent term color codes being sent to stdout
        // test: cat Cargo.toml | target/debug/hx | more
        // override via ARG_CLR below
        if !io::stdout().is_terminal() {
            colorize = false;
        }

        if let Some(color) = matches.get_one::<String>(ARG_CLR) {
            colorize = color.parse::<u8>().unwrap() == 1;
        }

        if let Some(prefix_flag) = matches.get_one::<String>(ARG_PFX) {
            prefix = prefix_flag.parse::<u8>().unwrap() == 1;
        }

        // array output mode is mutually exclusive
        if let Some(array) = matches.get_one::<String>(ARG_ARR) {
            output_array(array, buf, truncate_len, column_width)?;
        } else {
            // Transforms this Read instance to an Iterator over its bytes.
            // The returned type implements Iterator where the Item is
            // Result<u8, R::Err>. The yielded item is Ok if a byte was
            // successfully read and Err otherwise for I/O errors. EOF is
            // mapped to returning None from this iterator.
            // (https://doc.rust-lang.org/1.16.0/std/io/trait.Read.html#method.bytes)
            let mut ascii_line: Line = Line::new();
            let mut offset_counter: u64 = 0x0;
            let mut byte_column: u64 = 0x0;
            let page = buf_to_array(&mut buf, truncate_len, column_width)?;

            let stdout = io::stdout();
            let mut locked = stdout.lock();

            for line in page.body.iter() {
                print_offset(&mut locked, offset_counter)?;

                for hex in line.hex_body.iter() {
                    offset_counter += 1;
                    byte_column += 1;
                    print_byte(&mut locked, *hex, format_out, colorize, prefix)?;
                    append_ascii(&mut ascii_line.ascii, *hex, colorize);
                }

                if byte_column < column_width {
                    write!(
                        locked,
                        "{:<1$}",
                        "",
                        5 * (column_width - byte_column) as usize
                    )?;
                }

                locked.write_all(ascii_line.ascii.as_slice())?;
                writeln!(locked)?;

                byte_column = 0x0;
                ascii_line = Line::new();
            }
            if true {
                writeln!(locked, "   bytes: {}", page.bytes)?;
            }
        }
    }
    Ok(())
}

/// Detect stdin, file path and/or parameters.
/// # Arguments
///
/// * `matches` - argument matches.
#[allow(clippy::absurd_extreme_comparisons)]
pub fn is_stdin(matches: &ArgMatches) -> bool {
    if let Some(file) = matches.get_one::<String>(ARG_INP) {
        if DBG {
            dbg!(file);
        }

        return false;
    } else if let Some(nth1) = env::args().nth(1) {
        if DBG {
            dbg!(nth1);
        }

        return ARGS.iter().any(|arg| matches.index_of(arg) == Some(2));
    } else if !matches.args_present() {
        return true;
    }

    false
}

/// Output source code array format.
/// # Arguments
///
/// * `array_format` - array format, rust (r), C (c), golang (g).
/// * `buf` - BufRead.
/// * `truncate_len` - truncate to length.
/// * `column_width` - column width.
pub fn output_array(
    array_format: &str,
    mut buf: Box<dyn BufRead>,
    truncate_len: u64,
    column_width: u64,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut locked = stdout.lock();

    let page = buf_to_array(&mut buf, truncate_len, column_width).unwrap();
    match array_format {
        "r" => writeln!(locked, "let ARRAY: [u8; {}] = [", page.bytes)?,
        "c" => writeln!(locked, "unsigned char ARRAY[{}] = {{", page.bytes)?,
        "g" => writeln!(locked, "a := [{}]byte{{", page.bytes)?,
        "p" => writeln!(locked, "a = [")?,
        "k" => writeln!(locked, "val a = byteArrayOf(")?,
        "j" => writeln!(locked, "byte[] a = new byte[]{{")?,
        "s" => writeln!(locked, "let a: [UInt8] = [")?,
        "f" => writeln!(locked, "let a = [|")?,
        _ => writeln!(locked, "unknown array format")?,
    }
    let mut i: u64 = 0x0;
    for line in page.body.iter() {
        write!(locked, "    ")?;
        for hex in line.hex_body.iter() {
            i += 1;
            if i == page.bytes && array_format != "g" {
                if array_format != "f" {
                    write!(locked, "{}", Format::LowerHex.format(*hex, true))?;
                } else {
                    write!(locked, "{}uy", Format::LowerHex.format(*hex, true))?;
                }
            } else if array_format != "f" {
                write!(locked, "{}, ", Format::LowerHex.format(*hex, true))?;
            } else {
                write!(locked, "{}uy; ", Format::LowerHex.format(*hex, true))?;
            }
        }
        writeln!(locked)?;
    }

    writeln!(
        locked,
        "{}",
        match array_format {
            "r" => "];",
            "c" | "j" => "};",
            "g" => "}",
            "p" => "]",
            "k" => ")",
            "s" => "]",
            "f" => "|]",
            _ => "unknown array format",
        }
    )
}

/// Function wave out.
/// # Arguments
///
/// * `len` - Wave length.
/// * `places` - Number of decimal places for function wave floats.
pub fn output_function(len: u64, places: usize) {
    for y in 0..len {
        let y_float = y as f64;
        let len_float = len as f64;
        let x = (((y_float / len_float) * f64::consts::PI) / 2.0).sin();
        let formatted_number = format!("{:.*}", places, x);
        print!("{}", formatted_number);
        print!(",");
        if (y % 10) == 9 {
            println!();
        }
    }
    println!();
}

/// Buffer to array.
///
/// # Arguments
///
/// * `buf` - Buffer to be read.
/// * `buf_len` - force buffer length.
/// * `column_width` - column width for output.
pub fn buf_to_array(
    buf: &mut dyn Read,
    buf_len: u64,
    column_width: u64,
) -> Result<Page, Box<dyn ::std::error::Error>> {
    let mut column_count = 0u64;
    let max_array_size = u16::MAX; // 2^16;
    let mut page: Page = Page::new();
    let mut line: Line = Line::new();
    for b in buf.bytes() {
        let b1: u8 = b?;
        line.bytes += 1;
        page.bytes += 1;
        line.hex_body.push(b1);
        column_count += 1;

        if column_count >= column_width {
            page.body.push(line);
            line = Line::new();
            column_count = 0;
        }

        if buf_len > 0 && (page.bytes == buf_len || u64::from(max_array_size) == buf_len) {
            break;
        }
    }
    page.body.push(line);
    Ok(page)
}
