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
        writeln!(f, "AssemblyError: {}", self.0)
    }
}


/// Takes an instruction and returns a result containing either any immediate it finds if successful, or an error if it could not find one.
///
/// Panics if an immediate outside the valid range is found.
fn get_imm_from_instr(instr:String, bits:u32, signed:bool) -> Result<i16, Box<dyn Error>> {
    lazy_static! {
        static ref IMM_REGEX:Regex = Regex::new(r"[[:blank:]](0b[01]+|0x[[:xdigit:]]+|((\+|-)?[0-9]+))").unwrap();
    }

    let imm:i64;
    let imm_str:&str = IMM_REGEX.find_iter(&instr).map(|num| num.as_str()).collect::<Vec<&str>>()[0].trim();
    if imm_str.contains("0x") {  // hexadecimal number
        imm = i64::from_str_radix(imm_str.trim_start_matches("0x"), 16).unwrap();
    } else if imm_str.contains("0b") { // binary number
        imm = i64::from_str_radix(imm_str.trim_start_matches("0b"), 2).unwrap();
    } else {
        imm = imm_str.parse().unwrap();
    }

    if !signed && (imm < 0 || imm > 2_i64.pow(bits) - 1) {
        return Err(Box::new(AssemblyError(format!("Found negative immediate {} in unsigned immediate field in instruction {}", imm, instr))));
    } else if signed && (imm < -(2_i64.pow(bits) / 2) || imm > (2_i64.pow(bits) / 2) - 1) {
        return Err(Box::new(AssemblyError(format!("Found immediate {} outside valid range in instruction {}", imm, instr))));
    }

    return Ok(imm as i16)
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
            static ref RI_REGEX:Regex  = Regex::new(r"^([a-zA-Z]+:)?([[:blank:]]*)LUI[[:blank:]]*(((\$r[0-6]),)[[:blank:]]*)(0*([0-9]+|0b[01]+|0x[[:xdigit:]]+))[[:blank:]]*(#[[:blank:]]*[[:print:]]+)?$").unwrap();
            static ref JAL_REGEX:Regex = Regex::new(r"^([a-zA-Z]+:)?([[:blank:]]*)JAL[[:blank:]]*(\$(zero|r[0-6]),)[[:blank:]]*(\$(zero|r[0-6]))[[:blank:]]*(#[[:print:]]*)?$").unwrap();
            static ref NOP_REGEX:Regex = Regex::new(r"^([[:blank:]]*)([a-zA-Z]+:)?([[:blank:]]*)NOP([[:blank:]]*)(#[[:print:]]*)?$").unwrap();
            static ref DATA_REGEX:Regex = Regex::new(r"^([[:blank:]]*)([a-zA-Z]+:)?([[:blank:]]*)(LLI|MOVI)([[:blank:]]*)(\$r[0-6]),([[:blank:]]*)(0*([0-9]+|0b[01]+|0x[[:xdigit:]]+))([[:blank:]]*)(#[[:print:]]*)?$").unwrap();
        }

        if RRR_REGEX.is_match(&line) {
            continue;
        } else if RRI_REGEX.is_match(&line) {
            get_imm_from_instr(line, 7, true).unwrap();
            continue;
        } else if RI_REGEX.is_match(&line) {
            get_imm_from_instr(line, 10, false).unwrap();
            continue;
        } else if JAL_REGEX.is_match(&line) {
            continue;
        } else if NOP_REGEX.is_match(&line) {
            continue;
        } else if DATA_REGEX.is_match(&line) {
            if line.contains("LLI") {
                get_imm_from_instr(line, 6, false).unwrap();
            } else if line.contains("MOVI") {
                get_imm_from_instr(line, 16, true).unwrap();
            }

            continue;
        } else {
            return Err(Box::new(AssemblyError(format!("Line did not match any valid instructions patterns: {}", line))));
        }
    }

    Ok(())
}


/// Iterates through each line in the given file and returns a vector containing all the lines, then removes any '#' symbols and everythig after them, and finally
/// trims the resulting string.
/// 
/// Panics if a line cannot be read or the file cannot be found.
fn get_line_vector(filename: &str) -> Vec<String> {
    let input_file = OpenOptions::new().read(true).open(filename).expect(&format!("ERROR: Could not open file: {}", filename));
    let reader = BufReader::new(input_file);
    let lines:Vec<String> = {
        let mut line_num = 0;
        let mut result:Vec<String> = Vec::new();

        for line in reader.lines() {
            let mut ln = line.expect(&format!("ERROR: Could not read line {}", line_num)).trim().to_owned();
            ln = ln[..ln.find('#').unwrap_or(ln.len())].trim().to_owned(); // strip comments out of all lines

            result.push(ln);
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
        assert_eq!(lines[0], "ADDI $r0, $r0, 5");
        assert_eq!(lines[1], "ADDI $r0, $r1, 2");
        assert_eq!(lines[2], "NAND $r0, $r0, $r0");
        assert_eq!(lines[3], "NOP");
        assert_eq!(lines[4], "ADDI $r0, $r6, 1");
        assert_eq!(lines[5], "ADD $r0, $r0, $r1");
    }


    #[test]
    fn test_valid_file() {
        let lines = get_line_vector("test_files/test_line_vec_gen.asm");
        validate_assembly_lines(lines).unwrap();
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

    #[test]
    fn test_get_imm_from_instr() {
        let mut imm = get_imm_from_instr("ADDI $r0, $r1, 10".to_owned(), 7, true).unwrap();
        assert_eq!(imm, 10);

        imm = get_imm_from_instr("ADDI $r0, $r1, -10".to_owned(), 7, true).unwrap();
        assert_eq!(imm, -10);

        imm = get_imm_from_instr("ADDI $r0, $r1, 0x03A".to_owned(), 7, true).unwrap();
        assert_eq!(imm, 0x3A);

        imm = get_imm_from_instr("ADDI $r0, $r1, 0b011010".to_owned(), 7, true).unwrap();
        assert_eq!(imm, 0b11010);
    }


    #[test]
    #[should_panic]
    fn test_negative_unsigned_imm() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, -10".to_owned(), 7, false).unwrap();
    }


    #[test]
    #[should_panic]
    fn unsigned_imm_out_of_range() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, 128".to_owned(), 7, false).unwrap();
    }


    #[test]
    #[should_panic]
    fn signed_imm_to_large() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, 64".to_owned(), 7, true).unwrap();
    }


    #[test]
    #[should_panic]
    fn signed_imm_to_small() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, -65".to_owned(), 7, true).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_unsigned_imm_too_large_from_file() {
        let lines = get_line_vector("test_files/test_unsigned_imm_too_small.asm");
        validate_assembly_lines(lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_signed_imm_too_large_from_file() {
        let lines = get_line_vector("test_files/test_signed_imm_too_large.asm");
        validate_assembly_lines(lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_signed_imm_too_small_from_file() {
        let lines = get_line_vector("test_files/test_signed_imm_too_small.asm");
        validate_assembly_lines(lines).unwrap();
    }
}
