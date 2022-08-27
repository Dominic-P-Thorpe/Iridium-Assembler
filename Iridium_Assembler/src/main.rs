use std::{ env, fmt };
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{ BufReader, BufRead };
use lazy_static::lazy_static;
use regex::Regex;


#[derive(Debug)]
struct AssemblyError(String);

impl Error for AssemblyError {}
impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "AssmblyError: {}", self.0)
    }
}


/// Go line-by-line through each instruction in the file, skips if it is empty, and otherwise compares against a set of regular expressions to determine the type of
/// the instruction or pseudo-instruction, then performs other checks such as validating the range of immediate values.
///
/// Panics if an invalid instruction is found, otherwise returns `Ok()`
fn validate_assembly_lines(lines:Vec<String>) -> Result<(), Box<dyn Error>> {
    for line in lines {
        if line.is_empty() {
            continue;
        }

        lazy_static! {
            static ref RRR_REGEX:Regex = Regex::new(r"^([a-zA-Z]+:)?([[:blank:]]*)(ADD|NAND|BEQ)[[:blank:]]+(((\$(r[0-6])),)([[:blank:]]*))(((\$(zero|r[0-6])),)([[:blank:]]*))(\$(zero|r[0-6]))([[:blank:]]*)(#([[:blank:]]*)[[:print:]]+)?$").unwrap();
            static ref RRI_REGEX:Regex = Regex::new(r"^([a-zA-Z]+:)?([[:blank:]]*)(ADDI|SW|LW|JAL)[[:blank:]]+(((\$r[0-6]),)[[:blank:]]*)(((\$(zero|r[0-6])),)[[:blank:]]*)(0*((-|\+)?[0-9]+|0b[01]+|0x[[:xdigit:]]+))[[:blank:]]*(#[[:blank:]]*[[:print:]]+)?$").unwrap();
            static ref RI_REGEX:Regex = Regex::new(r"^([a-zA-Z]+:)?([[:blank:]]*)LUI[[:blank:]]*(((\$r[0-6]),)[[:blank:]]*)(0*([0-9]+|0b[01]+|0x[[:xdigit:]]+))[[:blank:]]*(#[[:blank:]]*[[:print:]]+)?$").unwrap();
            static ref JAL_REGEX:Regex = Regex::new(r"^([a-zA-Z]+:)?([[:blank:]]*)JAL[[:blank:]]*(\$(zero|r[0-6]),)[[:blank:]]*(\$(zero|r[0-6]))[[:blank:]]*(#[[:print:]]*)?$").unwrap();
        }

        if RRR_REGEX.is_match(&line) {
            continue;
        } else if RRI_REGEX.is_match(&line) {
            // Add validation for the signed immediate
            continue;
        } else if RI_REGEX.is_match(&line) {
            // Add validation for the unsigned immediate
            continue;
        } else if JAL_REGEX.is_match(&line) {
            continue;
        } else {
            return Err(Box::new(AssemblyError(format!("Line did not match any valid instructions patterns: {}", line))));
        }
    }

    Ok(())
}


/// Iterates through each line in the given file and returns a vector containing all the lines.
/// 
/// Panics if a line cannot be read or the file cannot be found.
fn get_line_vector(filename: &str) -> Vec<String> {
    let input_file = OpenOptions::new().read(true).open(filename).expect(&format!("ERROR: Could not open file: {}", filename));
    let reader = BufReader::new(input_file);
    let lines:Vec<String> = {
        let mut line_num = 0;
        let mut result:Vec<String> = Vec::new();

        for line in reader.lines() {
            result.push(line.expect(&format!("ERROR: Could not read line {}", line_num)).trim().to_owned());
            line_num += 1;
        }

        result
    };

    lines
}


fn main() {
    let args:Vec<String> = env::args().collect();
    println!("Assembling {} --> {}", args[1], args[2]);

    let lines:Vec<String> = get_line_vector(&args[1]);
    validate_assembly_lines(lines).unwrap();
}


#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_line_vector_generation() {
        let lines = get_line_vector("test_files/test_line_vec_gen.asm");
        assert_eq!(lines[0], "ADDI $r0 $r0 5");
        assert_eq!(lines[1], "ADDI $r0 $r1 2");
        assert_eq!(lines[2], "NAND $r0 $r0 $r0");
        assert_eq!(lines[3], "ADDI $r0 1");
        assert_eq!(lines[4], "ADD $r0 $r0 $r1");
    }


    #[test]
    #[should_panic]
    fn test_line_vector_gen_invalid_file() {
        let _lines = get_line_vector("test_files/does_not_exist.asm");
    }


    #[test]
    fn test_valid_instrs() {
        let lines = get_line_vector("test_files/test_valid_instrs.asm");
        validate_assembly_lines(lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_invalid_rrr() {
        let lines = get_line_vector("test_files/test_invalid_RRR.asm");
        validate_assembly_lines(lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_write_to_zero_reg() {
        let lines = get_line_vector("test_files/test_invalid_RRR.asm");
        validate_assembly_lines(lines).unwrap();
    }
}
