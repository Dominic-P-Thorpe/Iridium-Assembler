use std::{ env, fmt };
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{ BufReader, BufRead };
use lazy_static::lazy_static;
use regex::Regex;
use ascii_converter::string_to_decimals;


#[derive(Debug)]
struct AssemblyError(String);

impl Error for AssemblyError {}
impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "AssemblyError: {}", self.0)
    }
}


/// Takes a vector of instructions and examines it for any pseudo-instructions. If it finds any, then it replaces it with 1-or-more regular instructions which are inserted
/// into the vector in its place. The vector at the end of this process is returned.
fn substitute_pseudoinstrs(lines:&Vec<String>) -> Vec<String> {
    lazy_static! {
        static ref LABEL_REGEX:Regex = Regex::new(r"^[a-zA-Z_]+:").unwrap();
        static ref REGISTER_REGEX:Regex = Regex::new(r"\$r([0-6]|zero)").unwrap();
        static ref ELEM_REGEX:Regex = Regex::new(r"0b[01]+|0x[[:xdigit:]]+|((\+|-)?[0-9]+|'[[:ascii:]]')").unwrap();
    }

    let mut new_vec = lines.clone();
    let mut index:usize = 0;
    while index < new_vec.len() {
        let instr = new_vec[index].to_owned();
        let label = match LABEL_REGEX.find(&instr) {
            Some(val) => val.as_str().to_owned() + " ",
            None => "".to_owned()
        };

        if instr.contains("NOP") {
            new_vec.remove(index);
            new_vec.insert(index, format!("{}ADD $zero, $zero, $zero", label));
        } else if instr.contains("LLI") {
            let imm = get_imm_from_instr(&instr, 6, false, false).unwrap();
            let register = REGISTER_REGEX.find(&instr).unwrap().as_str();

            new_vec.remove(index);
            new_vec.insert(index, format!("{0}ADDI {1}, {1}, {2}", label, register, imm));
        } else if instr.contains("MOVI") {
            let register = REGISTER_REGEX.find(&instr).unwrap().as_str();
            let imm:u16 = get_imm_from_instr(&instr, 16, false, false).unwrap() as u16;
            let lower_imm = imm & 0x003F;
            let upper_imm = imm & 0xFFC0;

            new_vec.remove(index);
            new_vec.insert(index, format!("{0}ADDI {1}, {1}, {2}", label, register, lower_imm));
            new_vec.insert(index + 1, format!("LUI {}, {}", register, upper_imm));

            index += 1;
        } else if instr.contains(".space") {
            new_vec.remove(index);
            
            let defined_elems:Vec<u16> = ELEM_REGEX.find_iter(&instr).map(|item| convert_to_i64(item.as_str()).unwrap() as u16).collect::<Vec<u16>>()[1..].to_vec();
            let total_elems = ELEM_REGEX.find_iter(&instr).map(|item| convert_to_i64(item.as_str()).unwrap() as u16).collect::<Vec<u16>>()[0];

            for elem_index in 0..total_elems {
                let mut value_to_insert = format!("0x{:04X}", 0);
                if elem_index < defined_elems.len() as u16 {
                    value_to_insert = format!("0x{:04X}", defined_elems[elem_index as usize]);
                }

                if elem_index == 0 {
                    value_to_insert = label.to_owned() + &value_to_insert;
                }

                new_vec.insert(index + elem_index as usize, value_to_insert);
            }

            index += total_elems as usize - 1;
        } else if instr.contains(".text") {

        }

        index += 1;
    }

    new_vec
}


/// Takes a string formatted either as a decimal (signed or unsigned), binary (prefixed with "0b"), or hexadecimal (prefixed with "0x"), and outputs it as an `i64`. It
/// may also take a character as an input which conforms to the RegEx r"^'[[:ascii:]]'$" and will output the ASCII value of that character.
///
/// Returns an error if the value passed is not a decimal, hexadecimal, or binary integer or not a single character in single quotes.
fn convert_to_i64(raw_string:&str) -> Result<i64, Box<dyn Error>> {
    let imm:i64;
    if raw_string.contains("0x") {  // hexadecimal number
        imm = match i64::from_str_radix(raw_string.trim_start_matches("0x"), 16) {
            Ok(val) => val,
            Err(_) => { return Err(Box::new(AssemblyError(format!("Could not convert from {} to i64", raw_string)))) }
        };
    } else if raw_string.contains("0b") { // binary number
        imm = match i64::from_str_radix(raw_string.trim_start_matches("0b"), 2) {
            Ok(val) => val,
            Err(_) => { return Err(Box::new(AssemblyError(format!("Could not convert from {} to i64", raw_string)))) }
        };
    } else {
        imm = match raw_string.parse() {
            Ok(val) => val,
            Err(_) => match string_to_decimals(&raw_string[1..2]) {
                Ok(val) => *val.get(0).unwrap() as i64,
                Err(_) => { return Err(Box::new(AssemblyError(format!("Could not convert from {} to i64", raw_string)))) }
            }
        };
    }

    Ok(imm)
}


/// Takes an instruction and returns a result containing either any immediate it finds if successful, or an error if it could not find one.
///
/// Panics if an immediate outside the valid range is found.
fn get_imm_from_instr(instr:&str, bits:u32, signed:bool, accept_char:bool) -> Result<i16, Box<dyn Error>> {
    lazy_static! {
        static ref INT_REGEX:Regex = Regex::new(r"[[:blank:]](0b[01]+|0x[[:xdigit:]]+|((\+|-)?[0-9]+))").unwrap();
        static ref CHAR_REGEX:Regex = Regex::new(r"'[[:ascii:]]'").unwrap();
    }

    let imm_str:&str = match INT_REGEX.find_iter(&instr).map(|num| num.as_str()).collect::<Vec<&str>>().get(0) {
        Some(val) => val.trim(),
        None => {
            if !accept_char {
                return Err(Box::new(AssemblyError(format!("Could not find a valid immediate in instruction {}", instr))))
            }

            match CHAR_REGEX.find_iter(&instr).map(|num| num.as_str()).collect::<Vec<&str>>().get(0) {
                Some(val) => return Ok(*string_to_decimals(&val[1..2]).unwrap().get(0).unwrap() as i16),
                None      => return Err(Box::new(AssemblyError(format!("Could not find a valid immediate in instruction {}", instr))))
            }
        }
    };

    let imm:i64 = convert_to_i64(imm_str).unwrap();

    if !signed && (imm < 0 || imm > 2_i64.pow(bits) - 1) {
        return Err(Box::new(AssemblyError(format!("Found negative immediate {} in unsigned immediate field in instruction {}", imm, instr))));
    } else if signed && (imm < -(2_i64.pow(bits) / 2) || imm > (2_i64.pow(bits) / 2) - 1) {
        return Err(Box::new(AssemblyError(format!("Found immediate {} outside valid range in instruction {}", imm, instr))));
    }

    return Ok(imm as i16)
}


/// Validating .space will not work with the get_imm_from_instr() function due to Rust RegEx not implementing lookarounds. Therefore, this function validates them instead.
///
/// Panics if the input is not a valid statement.
fn validate_space(instr:&str) -> Result<(), Box<dyn Error>> {
    lazy_static! {
        static ref ELEM_REGEX:Regex = Regex::new(r"0b[01]+|0x[[:xdigit:]]+|((\+|-)?[0-9]+|'[[:ascii:]]')").unwrap();
    }

    let elems:Vec<&str> = ELEM_REGEX.find_iter(instr).map(|item| item.as_str()).collect();
    let array_len:i64 = elems.get(0).unwrap().parse().expect(&format!("Could not get length of array in instruction {}", instr));
    if elems.len() > array_len as usize {
        return Err(Box::new(AssemblyError(format!("Array is not long enough for data in instruction {}", instr))));
    }

    for elem in elems {
        let val = match elem.parse::<i64>() {
            Ok(val) => val,
            Err(_) => {
                let val:i64;
                if elem.contains("0x") {  // hexadecimal number
                    val = i64::from_str_radix(elem.trim_start_matches("0x"), 16).unwrap();
                } else if elem.contains("0b") { // binary number
                    val = i64::from_str_radix(elem.trim_start_matches("0b"), 2).unwrap();
                } else { // elem is a character
                    continue;
                }

                val
            }
        };

        if val > 65535 {
            return Err(Box::new(AssemblyError(format!("Value {} is out of the range 0 <= value < 65536 in instruction {}", val, instr).to_owned())));
        }
    }

    Ok(())
}


/// Go line-by-line through each instruction in the file, skips if it is empty, and otherwise compares against a set of regular expressions to determine the type of
/// the instruction or pseudo-instruction, then performs other checks such as validating the range of immediate values.
///
/// Panics if an invalid instruction is found, otherwise returns `Ok()`
fn validate_assembly_lines(lines:&Vec<String>) -> Result<(), Box<dyn Error>> {
    for line in lines {
        if line.is_empty() {
            continue;
        }

        lazy_static! {
            static ref UINT_REGEX:Regex  = Regex::new(r"0b[01]+|0x[[:xdigit:]]+|([0-9]+)").unwrap();
            static ref RRR_REGEX:Regex   = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)(ADD|NAND|BEQ)[[:blank:]]+(((\$(r[0-6])),)([[:blank:]]*))(((\$(zero|r[0-6])),)([[:blank:]]*))(\$(zero|r[0-6]))([[:blank:]]*)(#([[:blank:]]*)[[:print:]]+)?$").unwrap();
            static ref RRI_REGEX:Regex   = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)(ADDI|SW|LW|JAL)[[:blank:]]+(((\$r[0-6]),)[[:blank:]]*)(((\$(zero|r[0-6])),)[[:blank:]]*)(0*((-|\+)?[0-9]+|0b[01]+|0x[[:xdigit:]]+))[[:blank:]]*(#[[:blank:]]*[[:print:]]+)?$").unwrap();
            static ref RI_REGEX:Regex    = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)LUI[[:blank:]]*(((\$r[0-6]),)[[:blank:]]*)(0*([0-9]+|0b[01]+|0x[[:xdigit:]]+))[[:blank:]]*(#[[:blank:]]*[[:print:]]+)?$").unwrap();
            static ref JAL_REGEX:Regex   = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)JAL[[:blank:]]*(\$(zero|r[0-6]),)[[:blank:]]*(\$(zero|r[0-6]))[[:blank:]]*(#[[:print:]]*)?$").unwrap();
            static ref NOP_REGEX:Regex   = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)NOP([[:blank:]]*)(#[[:print:]]*)?$").unwrap();
            static ref DATA_REGEX:Regex  = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)(LLI|MOVI)([[:blank:]]*)(\$r[0-6]),([[:blank:]]*)(0*([0-9]+|0b[01]+|0x[[:xdigit:]]+))([[:blank:]]*)(#[[:print:]]*)?$").unwrap();
            static ref FILL_REGEX:Regex  = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*).fill[[:blank:]]*('[[:ascii:]]'|(0*((\+|-)?[0-9]+|0b[01]+|0x[[:xdigit:]]+)))([[:blank:]]*)(#[[:print:]]*)?$").unwrap();
            static ref SPACE_REGEX:Regex = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*).space[[:blank:]]+[0-9]+[[:blank:]]+\[([[:blank:]]*((\+|-)?[0-9]+|0x[[:xdigit:]]+|0b[01]+|'[[:ascii:]]'),[[:blank:]]*)*([0-9]+|0x[[:xdigit:]]+|0b[01]+|'[[:ascii:]]')?][[:blank:]]*(#[[:print:]]+)?$").unwrap();
            static ref TEXT_REGEX:Regex  = Regex::new(r#"^([a-zA-Z_]+:)?([[:blank:]]*).text[[:blank:]]+"[[:ascii:]]+"$"#).unwrap();
            static ref SCALL_REGEX:Regex = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*).syscall [0-7]$").unwrap();
        }

        if RRR_REGEX.is_match(&line) {
            continue;
        } else if RRI_REGEX.is_match(&line) {
            get_imm_from_instr(line, 7, true, false).unwrap();
            continue;
        } else if RI_REGEX.is_match(&line) {
            get_imm_from_instr(line, 10, false, false).unwrap();
            continue;
        } else if JAL_REGEX.is_match(&line) {
            continue;
        } else if NOP_REGEX.is_match(&line) {
            continue;
        } else if DATA_REGEX.is_match(&line) {
            if line.contains("LLI") {
                get_imm_from_instr(line, 6, false, false).unwrap();
            } else if line.contains("MOVI") {
                get_imm_from_instr(line, 16, false, false).unwrap();
            }

            continue;
        } else if FILL_REGEX.is_match(&line) {
            get_imm_from_instr(line, 16, true, true).unwrap();
            continue;
        } else if SPACE_REGEX.is_match(&line) {
            validate_space(&line).unwrap();
            continue;
        } else if TEXT_REGEX.is_match(&line) {
            continue;
        } else if SCALL_REGEX.is_match(&line) {
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

    let mut lines:Vec<String> = get_line_vector(&args[1]);
    validate_assembly_lines(&lines).unwrap();
    lines = substitute_pseudoinstrs(&lines);

    let mut index = 0;
    for line in lines {
        println!("{:04X}: {}", index, line);
        index += 1;
    }
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
        validate_assembly_lines(&lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_line_vector_gen_invalid_file() {
        let _lines = get_line_vector("test_files/does_not_exist.asm");
    }


    #[test]
    fn test_valid_instrs() {
        let lines = get_line_vector("test_files/test_valid_instrs.asm");
        validate_assembly_lines(&lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_invalid_rrr() {
        let lines = vec!["ADD $zero $r1 $r1".to_owned()];
        validate_assembly_lines(&lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_write_to_zero_reg() {
        let lines = vec!["ADD $zero, $r1, $r1".to_owned()];
        validate_assembly_lines(&lines).unwrap();
    }

    #[test]
    fn test_get_imm_from_instr() {
        let mut imm = get_imm_from_instr("ADDI $r0, $r1, 10", 7, true, true).unwrap();
        assert_eq!(imm, 10);

        imm = get_imm_from_instr("ADDI $r0, $r1, -10", 7, true, true).unwrap();
        assert_eq!(imm, -10);

        imm = get_imm_from_instr("ADDI $r0, $r1, 0x03A", 7, true, true).unwrap();
        assert_eq!(imm, 0x3A);

        imm = get_imm_from_instr("ADDI $r0, $r1, 0b011010", 7, true, true).unwrap();
        assert_eq!(imm, 0b11010);

        imm = get_imm_from_instr(".fill 'a'", 16, true, true).unwrap();
        assert_eq!(imm, 97);
    }


    #[test]
    #[should_panic]
    fn test_negative_unsigned_imm() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, -10", 7, false, false).unwrap();
    }


    #[test]
    #[should_panic]
    fn unsigned_imm_out_of_range() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, 128", 7, false, false).unwrap();
    }


    #[test]
    #[should_panic]
    fn signed_imm_to_large() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, 64", 7, true, false).unwrap();
    }


    #[test]
    #[should_panic]
    fn signed_imm_too_small() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, -65", 7, true, false).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_unsigned_imm_too_large() {
        let lines = vec!["ADDI $r0, $r1, 100000".to_owned()];
        validate_assembly_lines(&lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_signed_imm_too_large() {
        let lines = vec!["ADDI $r0, $r1, 100".to_owned()];
        validate_assembly_lines(&lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_non_ascii_char_fill() {
        let lines = vec![".fill 'д'".to_owned()];
        validate_assembly_lines(&lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_invalid_fill_integer() {
        let lines = vec![".fill -100000".to_owned()];
        validate_assembly_lines(&lines).unwrap();
    }


    #[test]
    fn test_validate_space() {
        validate_space(".space 10 [100, 200, 0xFF, 0b001100, 'a', 'b']").unwrap();
    }


    #[test]
    #[should_panic]
    fn test_validate_invalid_space() {
        validate_space(".space 10 [100, 200, 0xFFFFF, 0b001100, 'a', 'b']").unwrap();
    }


    #[test]
    #[should_panic]
    fn test_array_too_small() {
        validate_space(".space 3 [100, 200, 50, 20]").unwrap();
    }


    #[test]
    #[should_panic]
    fn test_invalid_syscall_code() {
        let lines = vec![".syscall 18".to_owned()];
        validate_assembly_lines(&lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_label_with_space() {
        let lines = vec!["hello world: ADD $r0, $r1, $r2".to_owned()];
        validate_assembly_lines(&lines).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_label_with_non_alphabet_char() {
        let lines = vec!["he**world: ADD $r0, $r1, $r2".to_owned()];
        validate_assembly_lines(&lines).unwrap();
    }


    #[test]
    fn test_valid_pseudoinstr_substitutions() {
        let mut lines = get_line_vector("test_files/test_valid_pseudo_subs.asm");
        validate_assembly_lines(&lines).unwrap();
        lines = substitute_pseudoinstrs(&lines);

        assert_eq!(lines[0], "ADDI $r0, $zero, 20");
        assert_eq!(lines[1], "ADDI $r1, $r1, 20");
        assert_eq!(lines[2], "ADD $zero, $zero, $zero");
        assert_eq!(lines[3], "ADDI $r2, $zero, 20");
        assert_eq!(lines[4], "labelA: ADDI $r2, $r2, 50");
        assert_eq!(lines[5], "labelB: ADD $zero, $zero, $zero");
        assert_eq!(lines[6], "labelC: ADDI $r1, $r1, 48");
        assert_eq!(lines[7], "LUI $r1, 63488");
        assert_eq!(lines.len(), 8);
    }


    #[test]
    #[should_panic]
    fn test_invalid_lli() {
        let lines = vec!["LLI $r0, 86".to_owned()];
        validate_assembly_lines(&lines).unwrap();
    }


    #[test]
    fn test_convert_to_i64() {
        assert_eq!(convert_to_i64("100").unwrap(), 100);
        assert_eq!(convert_to_i64("-100").unwrap(), -100);
        assert_eq!(convert_to_i64("0x0F4").unwrap(), 244);
        assert_eq!(convert_to_i64("0b0110").unwrap(), 6);
        assert_eq!(convert_to_i64("'c'").unwrap(), 99);
        assert_eq!(convert_to_i64("'&''").unwrap(), 38);
    }


    #[test]
    #[should_panic]
    fn test_convert_to_i64_non_ascii_char() {
        assert_eq!(convert_to_i64("'Ж'").unwrap(), 100);
    }


    #[test]
    #[should_panic]
    fn test_convert_to_i64_malformed_char() {
        assert_eq!(convert_to_i64("a'").unwrap(), 100);
    }


    #[test]
    fn test_space_sub() {
        let mut lines = get_line_vector("test_files/test_space_sub.asm");
        validate_assembly_lines(&lines).unwrap();
        lines = substitute_pseudoinstrs(&lines);

        assert_eq!(lines[0], "ADD $r0, $r1, $r2");
        assert_eq!(lines[1], "start: 0x0064");
        assert_eq!(lines[2], "0xFFFE");
        assert_eq!(lines[3], "0x0061");
        assert_eq!(lines[4], "0x0000");
        assert_eq!(lines[5], "0x0000");
        assert_eq!(lines[6], "ADD $r0, $r1, $r3");
    }
}

