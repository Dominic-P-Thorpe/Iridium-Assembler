use std::{ env, fmt };
use std::collections::HashMap;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::{ BufReader, BufRead, Write };
use lazy_static::lazy_static;
use regex::Regex;
use ascii_converter::string_to_decimals;


lazy_static! {
    static ref RI_REGEX:Regex = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)LUI[[:blank:]]*(((\$(zero|r[0-6])),)[[:blank:]]*)(0*([0-9]+|0b[01]+|0x[[:xdigit:]]+|@[a-zA-Z_]+))[[:blank:]]*(#[[:blank:]]*[[:print:]]+)?$").unwrap();
    static ref RRR_REGEX:Regex = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)(ADD|NAND|BEQ)[[:blank:]]+(((\$(zero|r[0-6])),)([[:blank:]]*))(((\$(zero|r[0-6])),)([[:blank:]]*))(\$(zero|r[0-6]))([[:blank:]]*)(#([[:blank:]]*)[[:print:]]+)?$").unwrap();
    static ref RRI_REGEX:Regex = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)(ADDI|SW|LW|JAL)[[:blank:]]+(((\$(zero|r[0-6])),)[[:blank:]]*)(((\$(zero|r[0-6])),)[[:blank:]]*)(0*((-|\+)?[0-9]+|0b[01]+|0x[[:xdigit:]]+)|@[a-zA-Z_]+)[[:blank:]]*(#[[:blank:]]*[[:print:]]+)?$").unwrap();
    static ref JAL_REGEX:Regex = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)JAL[[:blank:]]*(\$(zero|r[0-6]),)[[:blank:]]*(\$(zero|r[0-6]))[[:blank:]]*(#[[:print:]]*)?$").unwrap();
    static ref NOP_REGEX:Regex = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)NOP([[:blank:]]*)(#[[:print:]]*)?$").unwrap();
    static ref INT_REGEX:Regex = Regex::new(r"[[:blank:]](0b[01]+|0x[[:xdigit:]]+|((\+|-)?[0-9]+))").unwrap();
    static ref ELEM_REGEX:Regex = Regex::new(r"0b[01]+|0x[[:xdigit:]]+|((\+|-)?[0-9]+|'[[:ascii:]]')").unwrap();
    static ref CHAR_REGEX:Regex = Regex::new(r"'[[:ascii:]]'").unwrap();
    static ref UINT_REGEX:Regex = Regex::new(r"0b[01]+|0x[[:xdigit:]]+|([0-9]+)").unwrap();
    static ref DATA_REGEX:Regex = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*)(LLI|MOVI)([[:blank:]]*)(\$(zero|r[0-6])),([[:blank:]]*)(0*([0-9]+|0b[01]+|0x[[:xdigit:]]+|@[a-zA-Z_]+))([[:blank:]]*)(#[[:print:]]*)?$").unwrap();
    static ref FILL_REGEX:Regex = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*).fill[[:blank:]]*('[[:ascii:]]'|(0*((\+|-)?[0-9]+|0b[01]+|0x[[:xdigit:]]+)))([[:blank:]]*)(#[[:print:]]*)?$").unwrap();
    static ref INSTR_REGEX:Regex = Regex::new("ADDI|NAND|LUI|SW|LW|BEQ|JAL|ADD|.syscall").unwrap();
    static ref SPACE_REGEX:Regex = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*).space[[:blank:]]+[0-9]+[[:blank:]]+\[([[:blank:]]*((\+|-)?[0-9]+|0x[[:xdigit:]]+|0b[01]+|'[[:ascii:]]'),[[:blank:]]*)*([0-9]+|0x[[:xdigit:]]+|0b[01]+|'[[:ascii:]]')?][[:blank:]]*(#[[:print:]]+)?$").unwrap();
    static ref SCALL_REGEX:Regex = Regex::new(r"^([a-zA-Z_]+:)?([[:blank:]]*).syscall [0-7]$").unwrap();
    static ref LABEL_REGEX:Regex = Regex::new(r"^[a-zA-Z_]+:").unwrap();
    static ref REGISTER_REGEX:Regex = Regex::new(r"\$(r[0-6]|zero)").unwrap();
    static ref TEXT_IMM_REGEX:Regex = Regex::new(r#""[[:ascii:]]+""#).unwrap();
    static ref LABEL_ARG_REGEX:Regex = Regex::new(r"@[a-zA-Z_]+").unwrap();
    static ref PSEUDO_TEXT_REGEX:Regex = Regex::new(r#"^([a-zA-Z_]+:)?([[:blank:]]*).text[[:blank:]]+"[[:ascii:]]+"$"#).unwrap();
}


#[derive(Debug)]
struct AssemblyError(String);

impl Error for AssemblyError {}
impl fmt::Display for AssemblyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "AssemblyError: {}", self.0)
    }
}


/// Takes a valid instruction and converts it to its binary equivalent as a byte, or returns an `AssemblyError` or panics if it cannot.
fn convert_instr_to_binary(instr:&String) -> Result<u16, Box<dyn Error>> {
    let opcodes = HashMap::from([
        ("ADD", 0x0000), ("ADDI", 0x2000), ("NAND", 0x4000), ("LUI", 0x6000), 
        ("SW",  0x8000), ("LW",   0xA000), ("BEQ",  0xC000), ("JAL", 0xE000),
        (".syscall", 0xE000)
    ]);

    let registers = HashMap::from([
        ("$zero", 0x00), ("$r0", 0x01), ("$r1", 0x02), ("$r2", 0x03), ("$r3", 0x04), ("$r4", 0x05), ("$r5", 0x06), ("$r6", 0x07)
    ]);
    
    // let opcode:u16 = match opcodes.get(INSTR_REGEX.find(instr).unwrap().as_str()) {
    let opcode:u16 = match INSTR_REGEX.find(instr) {
        Some(val) => *opcodes.get(val.as_str()).unwrap(),
        None => {
            if !UINT_REGEX.is_match(instr) {
                return Err(Box::new(AssemblyError(format!("{} is not a valid instruction for compilation. Note pseudoinstructions cannot be present at this stage", instr))));
            }

            let data_byte = get_imm_from_instr(&instr, 16, false, false, false)?.unwrap() as u16;
            return Ok(data_byte);
        }
    };

    let registers:Vec<u16> = REGISTER_REGEX.find_iter(&instr).map(|reg| *registers.get(reg.as_str()).unwrap() as u16).collect();
    let instr_binary = match opcode {
        0x0000 | 0x4000 | 0xC000 => {
            let mut result = opcode;
            if registers.len() != 3 {
                return Err(Box::new(AssemblyError(format!("{} does not have 3 registers as is required", instr))));
            }

            let (reg_a, reg_b, reg_c) = (
                registers[0] << 10,
                registers[1] << 7,
                registers[2] << 4
            );

            result |= reg_a;
            result |= reg_b;
            result |= reg_c;

            result
        },

        0x2000 | 0x8000 | 0xA000 => {
            let mut result = opcode;
            let immediate = get_imm_from_instr(instr, 7, true, false, false).unwrap().unwrap() as u16 & 0x007F;
            if registers.len() != 2 {
                return Err(Box::new(AssemblyError(format!("{} does not have 2 registers as is required", instr))));
            }

            let (reg_a, reg_b) = (
                registers[0] << 10,
                registers[1] << 7
            );

            result |= reg_a;
            result |= reg_b;
            result |= immediate;

            result
        }

        0x6000 => {
            let mut result = opcode;
            let immediate = get_imm_from_instr(instr, 10, false, false, false).unwrap().unwrap() as u16 & 0x03FF;
            let reg_a = registers[0] << 10;
            if registers.len() != 1 {
                return Err(Box::new(AssemblyError(format!("{} does not have 1 register as is required", instr))));
            }

            result |= reg_a;
            result |= immediate;

            result
        }

        0xE000 => {
            let mut result = opcode;
            if instr.contains(".syscall") {
                let immediate = get_imm_from_instr(instr, 7, false, false, false).unwrap().unwrap() as u16 & 0x007F;
                let reg_a = 0x1400; // 0b0001 0100 0000 0000

                result |= reg_a;
                result |= immediate;
            } 
            
            else {
                if registers.len() != 2 {
                    return Err(Box::new(AssemblyError(format!("{} does not have 2 registers as is required", instr))));
                }
    
                let (reg_a, reg_b) = (
                    registers[0] << 10,
                    registers[1] << 7
                );
    
                result |= reg_a;
                result |= reg_b;
            }

            result
        }

        _ => { 
            return Err(Box::new(AssemblyError(format!("{} does not contain a valid opcode", instr)))) 
        }
    };

    Ok(instr_binary)
}


/// Goes through every line of the program and checks for labels. If it finds a label, it will substitute in the appropriate value in its place.
///
/// WARNING: only works if the pseudo-instructions have already been substituted.
///
/// Panics if an undefined label is encountered.
fn substitute_labels(lines:&Vec<String>, label_table:&HashMap<String, i32>) -> Vec<String> {
    let mut new_lines:Vec<String> = Vec::new();
    for line in lines {
        let label:String = match LABEL_ARG_REGEX.find(line) {
            Some(val) => val.as_str().to_owned(),
            None => {
                new_lines.append(&mut vec![line.to_owned()]);
                continue;
            }
        };

        let mut address = *label_table.get(&label[1..]).expect(&format!("Could not find label {} in instruction {}", label, line));
        if line.contains("ADDI") || line.contains("LW") || line.contains("SW") {
            address = address & 0x003F;
        } else if line.contains("LUI") {
            address = (address & 0xFFC0) >> 6;
        }

        new_lines.append(&mut vec![line.replace(&label, &address.to_string()).to_owned()]);
    }

    new_lines
}


/// Goes through every line of the program looking for instructions with a label matching the regex `^[a-zA-Z_]+:`. This is then added to a `HashMap` with the label's
/// name as the key and its line number as the value - this hashmap is the return value.
fn generate_label_table(lines:&Vec<String>) -> Result<HashMap<String, i32>, Box<dyn Error>> {
    let mut label_table:HashMap<String, i32> = HashMap::new();
    let mut line_num = 0;
    for line in lines {
        match LABEL_REGEX.find(line) {
            Some(val) => { 
                let label_name = val.as_str().replace(":", "");
                if label_table.keys().collect::<Vec<&String>>().contains(&&label_name) {
                    return Err(Box::new(AssemblyError(format!("Found duplicate key {}", label_name))));
                }

                label_table.insert(label_name, line_num);
            },

            None => (),
        };
        
        line_num += 1;
    }

    Ok(label_table)
}


/// Takes an instruction and the valid number of bits the operand can have as arguments. Checks the instruction for any immediates in number, character, and label form and
/// returns them if there are any, or an `AssemblyError` if not. 
fn get_imm_for_pseudoinstr(instr:&String, bits:u32) -> Result<String, Box<dyn Error>> {
    let mut imm = None;
    let mut label = None;
    match get_imm_from_instr(&instr, bits, false, false, true).unwrap() {
        Some(val) => { imm = Some(val) },
        None => {
            label = Some (match LABEL_ARG_REGEX.find(&instr) {
                Some(val) => val.as_str(),
                None => { return Err(Box::new(AssemblyError(format!("Could not find valid immediate for instruction {}", instr)))) }
            });
        }
    };

    match imm {
        Some(val) => {
            return Ok(val.to_string());
        },

        None => {
            return Ok(label.expect(&format!("Could not find valid immediate for instruction {}", instr)).to_owned());
        }
    }
}


/// Takes a vector of instructions and examines it for any pseudo-instructions. If it finds any, then it replaces it with 1-or-more regular instructions which are inserted
/// into the vector in its place. The vector at the end of this process is returned.
fn substitute_pseudoinstrs(lines:&Vec<String>) -> Vec<String> {
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
            let imm = get_imm_for_pseudoinstr(&instr, 6).unwrap();
            let register = REGISTER_REGEX.find(&instr).unwrap().as_str();

            new_vec.remove(index);
            new_vec.insert(index, format!("{0}ADDI {1}, {1}, {2}", label, register, imm));
        } else if instr.contains("MOVI") {
            new_vec.remove(index);

            let register = REGISTER_REGEX.find(&instr).unwrap().as_str();
            let imm = get_imm_for_pseudoinstr(&instr, 16).unwrap();
            match convert_to_i64(&imm) {
                Ok(val) => {
                    let lower_imm = val as u16 & 0x003F;
                    let upper_imm = (val as u16 & 0xFFC0) >> 6;

                    new_vec.insert(index, format!("{}ADDI {}, $zero, {}", label, register, lower_imm));
                    new_vec.insert(index + 1, format!("LUI {}, {}", register, upper_imm));
                },

                Err(_) => {
                    println!("Imm: {}", imm);
                    new_vec.insert(index, format!("{}ADDI {}, $zero, {}", label, register, imm));
                    new_vec.insert(index + 1, format!("LUI {}, {}", register, imm));
                }
            };

            index += 1;
        } else if instr.contains(".space") {
            new_vec.remove(index);
            
            let defined_elems:Vec<u16> = ELEM_REGEX.find_iter(&instr).map(|item| convert_to_i64(item.as_str()).unwrap() as u16).collect::<Vec<u16>>()[1..].to_vec();
            let total_elems = ELEM_REGEX.find_iter(&instr).map(|item| convert_to_i64(item.as_str()).unwrap() as u16).collect::<Vec<u16>>()[0];

            for elem_index in 0..total_elems {
                let mut value_to_insert = format!(".fill 0x{:04X}", 0);
                if elem_index < defined_elems.len() as u16 {
                    value_to_insert = format!(".fill 0x{:04X}", defined_elems[elem_index as usize]);
                }

                if elem_index == 0 {
                    value_to_insert = label.to_owned() + &value_to_insert;
                }

                new_vec.insert(index + elem_index as usize, value_to_insert);
            }

            index += total_elems as usize - 1;
        } else if instr.contains(".text") {
            new_vec.remove(index);

            let text = TEXT_IMM_REGEX.find(&instr).unwrap().as_str();
            let cleaned_text = text[1..text.len() - 1].to_owned();
            let text_ascii = string_to_decimals(&cleaned_text).unwrap().into_iter().map(|item| format!(".fill 0x{:04X}", item)).collect::<Vec<String>>();

            let mut elem_index = 0;
            for mut char_str in text_ascii {
                if elem_index == 0 {
                    char_str = label.to_owned() + &char_str;
                }

                new_vec.insert(elem_index + index as usize, char_str);
                elem_index += 1;
            }

            new_vec.insert(elem_index + index as usize, ".fill 0x0000".to_owned());
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
            Err(_) => {
                if CHAR_REGEX.find(raw_string) == None {
                    return Err(Box::new(AssemblyError(format!("Could not convert from {} to i64", raw_string))))
                }

                match string_to_decimals(&raw_string[1..2]) {
                    Ok(val) => *val.get(0).unwrap() as i64,
                    Err(_) => { return Err(Box::new(AssemblyError(format!("Could not convert from {} to i64", raw_string)))) }
                }
            }
        };
    }

    Ok(imm)
}


/// Takes an instruction and returns a result containing either any immediate it finds if successful, or an error if it could not find one. If it finds a label immediate,
/// then it will return `None`.
///
/// Panics if an immediate outside the valid range is found.
fn get_imm_from_instr(instr:&str, bits:u32, signed:bool, accept_char:bool, accept_label:bool) -> Result<Option<i16>, Box<dyn Error>> {
    match LABEL_ARG_REGEX.find(&instr) {
        Some(val) => {
            if accept_label {
                return Ok(None);
            }

            return Err(Box::new(AssemblyError(format!("Found label {} in instruction {} but labels are not accepted", val.as_str(), instr))));
        },

        None => {}
    };

    // prepended space needed to ensure that regex can tell the difference between a number such as the one6 in "$r6" and an actual immediate as Rust Regex does not support
    // negative lookbehinds to check for "$r".
    let instr_with_prepended_space = " ".to_owned() + instr;

    let imm_str:&str = match INT_REGEX.find_iter(&instr_with_prepended_space).map(|num| num.as_str()).collect::<Vec<&str>>().get(0) {
        Some(val) => val.trim(),
        None => {
            if !accept_char {
                return Err(Box::new(AssemblyError(format!("Could not find a valid immediate in instruction {}", instr))))
            }

            match CHAR_REGEX.find_iter(&instr).map(|num| num.as_str()).collect::<Vec<&str>>().get(0) {
                Some(val) => return Ok(Some(*string_to_decimals(&val[1..2]).unwrap().get(0).unwrap() as i16)),
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

    return Ok(Some(imm as i16))
}


/// Validating .space will not work with the get_imm_from_instr() function due to Rust RegEx not implementing lookarounds. Therefore, this function validates them instead.
///
/// Panics if the input is not a valid statement.
fn validate_space(instr:&str) -> Result<(), Box<dyn Error>> {
    let elems:Vec<&str> = ELEM_REGEX.find_iter(instr).map(|item| item.as_str()).collect();
    let array_len:i64 = elems.get(0).unwrap().parse().expect(&format!("Could not get length of array in instruction {}", instr));
    if elems.len() > (array_len + 1) as usize {
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

        if RRR_REGEX.is_match(&line) {
            continue;
        } else if RRI_REGEX.is_match(&line) {
            get_imm_from_instr(line, 7, true, false, true).unwrap();
            continue;
        } else if RI_REGEX.is_match(&line) {
            get_imm_from_instr(line, 10, false, false, true).unwrap();
            continue;
        } else if JAL_REGEX.is_match(&line) {
            continue;
        } else if NOP_REGEX.is_match(&line) {
            continue;
        } else if DATA_REGEX.is_match(&line) {
            if line.contains("LLI") {
                get_imm_from_instr(line, 6, false, false, true).unwrap();
            } else if line.contains("MOVI") {
                get_imm_from_instr(line, 16, false, false, true).unwrap();
            }

            continue;
        } else if FILL_REGEX.is_match(&line) {
            get_imm_from_instr(line, 16, true, true, false).unwrap();
            continue;
        } else if SPACE_REGEX.is_match(&line) {
            validate_space(&line).unwrap();
            continue;
        } else if PSEUDO_TEXT_REGEX.is_match(&line) {
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


/// Takes a vector containing the processed and assembled instructions and writes them to the specified file as 2 bytes (16 bits), creating the file if it does not
/// already exist and then returns the number of bytes written.
fn write_assembled_bytes(filename: &str, instrs: Vec<u16>) -> usize {
    let mut output_file = OpenOptions::new().write(true).create(true).open(filename).expect(&format!("ERROR: Could not open file: {}", filename));

    let mut bytes:Vec<u8> = Vec::new();
    for instr in instrs {
        bytes.push(((instr & 0xFF00) >> 8) as u8);
        bytes.push((instr & 0x00FF) as u8);
    }

    output_file.write_all(&bytes.as_slice()).unwrap();
    return bytes.len();
}


fn main() {
    let args:Vec<String> = env::args().collect();
    println!("Assembling {} --> {}", args[1], args[2]);

    let mut lines:Vec<String> = get_line_vector(&args[1]);
    lines = lines.into_iter().filter(|line| !line.is_empty()).collect();
    validate_assembly_lines(&lines).unwrap();
    lines = substitute_pseudoinstrs(&lines);

    let label_table = generate_label_table(&lines).unwrap();
    lines = substitute_labels(&lines, &label_table);

    let mut assembled_lines = Vec::new();
    let mut index = 0;
    for line in lines {
        assembled_lines.push(convert_instr_to_binary(&line).unwrap());
        println!("0x{:04X}:\t {:32} \t 0x{:04X}", index, line, convert_instr_to_binary(&line).unwrap());
        index += 1;
    }

    let num_bytes = write_assembled_bytes(&args[2], assembled_lines);
    println!("Successfully assembled {} bytes", num_bytes);
}


#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_line_vector_generation() {
        let lines = get_line_vector("test_files/test_line_vec_gen.asm");
        assert_eq!(lines[0], "start: ADDI $r0, $r0, 5");
        assert_eq!(lines[1], "ADDI $r0, $r1, 2");
        assert_eq!(lines[2], "NAND $r0, $r0, $r0");
        assert_eq!(lines[3], "NOP");
        assert_eq!(lines[4], "ADDI $r0, $r6, 1");
        assert_eq!(lines[5], "ADD $r0, $r0, $r1");
        assert_eq!(lines[6], "MOVI $r0, @start");
        assert_eq!(lines.len(), 7);
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
    fn test_get_imm_from_instr() {
        let mut imm = get_imm_from_instr("ADDI $r0, $r1, 10", 7, true, true, true).unwrap();
        assert_eq!(imm.unwrap(), 10);

        imm = get_imm_from_instr("ADDI $r0, $r1, -10", 7, true, true, true).unwrap();
        assert_eq!(imm.unwrap(), -10);

        imm = get_imm_from_instr("ADDI $r0, $r1, 0x03A", 7, true, true, true).unwrap();
        assert_eq!(imm.unwrap(), 0x3A);

        imm = get_imm_from_instr("ADDI $r0, $r1, 0b011010", 7, true, true, true).unwrap();
        assert_eq!(imm.unwrap(), 0b11010);

        imm = get_imm_from_instr(".fill 'a'", 16, true, true, false).unwrap();
        assert_eq!(imm.unwrap(), 97);

        imm = get_imm_from_instr("ADDI $r0, $r1, @label", 16, true, true, true).unwrap();
        assert_eq!(imm, None);
    }


    #[test]
    #[should_panic]
    fn test_invalid_label_imm() {
        let imm = get_imm_from_instr("ADDI $r0, $r1, @label", 16, true, true, false).unwrap();
        assert_eq!(imm, None);
    }


    #[test]
    #[should_panic]
    fn test_negative_unsigned_imm() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, -10", 7, false, false, true).unwrap();
    }


    #[test]
    #[should_panic]
    fn unsigned_imm_out_of_range() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, 128", 7, false, false, true).unwrap();
    }


    #[test]
    #[should_panic]
    fn signed_imm_to_large() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, 64", 7, true, false, true).unwrap();
    }


    #[test]
    #[should_panic]
    fn signed_imm_too_small() {
        let _imm = get_imm_from_instr("ADDI $r0, $r1, -65", 7, true, false, true).unwrap();
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
        validate_assembly_lines(&lines).unwrap();

        assert_eq!(lines[0], "ADDI $r0, $zero, 20");
        assert_eq!(lines[1], "ADDI $r1, $r1, 20");
        assert_eq!(lines[2], "ADD $zero, $zero, $zero");
        assert_eq!(lines[3], "ADDI $r2, $zero, 20");
        assert_eq!(lines[4], "labelA: ADDI $r2, $r2, 50");
        assert_eq!(lines[5], "labelB: ADD $zero, $zero, $zero");
        assert_eq!(lines[6], "labelC: ADDI $r1, $zero, 48");
        assert_eq!(lines[7], "LUI $r1, 992");
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
        assert_eq!(lines[1], "start: .fill 0x0064");
        assert_eq!(lines[2], ".fill 0xFFFE");
        assert_eq!(lines[3], ".fill 0x0061");
        assert_eq!(lines[4], ".fill 0x0000");
        assert_eq!(lines[5], ".fill 0x0000");
        assert_eq!(lines[6], "ADD $r0, $r1, $r3");
    }


    #[test]
    fn test_text_sub() {
        let mut lines = vec!["tag: .text \"Hell@ \"w0rld!\"".to_owned()];
        validate_assembly_lines(&lines).unwrap();
        lines = substitute_pseudoinstrs(&lines);

        assert_eq!(lines[0], "tag: .fill 0x0048");
        assert_eq!(lines[2], ".fill 0x006C");
        assert_eq!(lines[4], ".fill 0x0040");
        assert_eq!(lines[5], ".fill 0x0020");
        assert_eq!(lines[6], ".fill 0x0022");
        assert_eq!(lines[12], ".fill 0x0021");
        assert_eq!(lines[13], ".fill 0x0000");
        assert_eq!(lines.len(), 14);
    }


    #[test]
    fn test_label_table_generation() {
        let mut lines = get_line_vector("test_files/test_label_table_generation.asm");
        validate_assembly_lines(&lines).unwrap();

        lines = substitute_pseudoinstrs(&lines);
        lines = lines.into_iter().filter(|line| !line.is_empty()).collect();
        
        let tags = generate_label_table(&lines).unwrap();
        assert_eq!(tags["start"], 0);
        assert_eq!(tags["something"], 3);
        assert_eq!(tags["number"], 4);
        assert_eq!(tags["hello"], 5);
        assert_eq!(tags["more_text"], 11);
    }


    #[test]
    #[should_panic]
    fn test_duplicate_label() {
        let mut lines = get_line_vector("test_files/test_duplicate_label.asm");
        validate_assembly_lines(&lines).unwrap();

        lines = substitute_pseudoinstrs(&lines);
        lines = lines.into_iter().filter(|line| !line.is_empty()).collect();

        generate_label_table(&lines).unwrap();
    }


    #[test]
    fn test_label_operands() {
        let mut lines:Vec<String> = get_line_vector("test_files/test_label_operands.asm");
        lines = lines.into_iter().filter(|line| !line.is_empty()).collect();
        validate_assembly_lines(&lines).unwrap();

        lines = substitute_pseudoinstrs(&lines);

        let label_table = generate_label_table(&lines).unwrap();
        lines = substitute_labels(&lines, &label_table);

        assert_eq!(lines[2], "move: ADDI $r6, $zero, 0");
        assert_eq!(lines[5], "ADDI $r0, $zero, 2");
        assert_eq!(lines[77], "after_text: ADDI $r6, $zero, 6");
        assert_eq!(lines[78], "LUI $r6, 0");
        assert_eq!(lines[79], "ADDI $r5, $zero, 13");
        assert_eq!(lines[80], "LUI $r5, 1");
    }


    #[test]
    #[should_panic]
    fn test_non_existent_label_operand() {
        let mut _lines = vec!["MOVI $r1, @nowhere".to_owned()];
        _lines = _lines.into_iter().filter(|line| !line.is_empty()).collect();
        validate_assembly_lines(&_lines).unwrap();

        _lines = substitute_pseudoinstrs(&_lines);

        let label_table = generate_label_table(&_lines).unwrap();
        _lines = substitute_labels(&_lines, &label_table);
    }


    #[test]
    fn test_convert_to_binary() {
        assert_eq!(convert_instr_to_binary(&"ADD  $r0, $zero, $r1".to_owned()).unwrap(), 0x0420_u16);
        assert_eq!(convert_instr_to_binary(&"NAND $r2, $r3,   $r4".to_owned()).unwrap(), 0x4E50_u16);
        assert_eq!(convert_instr_to_binary(&"BEQ  $r5, $zero, $r6".to_owned()).unwrap(), 0xD870_u16);

        assert_eq!(convert_instr_to_binary(&"ADDI $r1, $zero,  7".to_owned()).unwrap(),  0x2807_u16);
        assert_eq!(convert_instr_to_binary(&"ADDI $r1, $zero, -7".to_owned()).unwrap(),  0x2879_u16);
        assert_eq!(convert_instr_to_binary(&"SW   $r1, $r2,   30".to_owned()).unwrap(),  0x899E_u16);
        assert_eq!(convert_instr_to_binary(&"LW   $r6, $r5,  -10".to_owned()).unwrap(),  0xBF76_u16);

        assert_eq!(convert_instr_to_binary(&"0x0455".to_owned()).unwrap(), 0x0455_u16);
        assert_eq!(convert_instr_to_binary(&"10000".to_owned()).unwrap(),  0x2710_u16);

        assert_eq!(convert_instr_to_binary(&"LUI $r0, 500".to_owned()).unwrap(),  0x65F4_u16);

        assert_eq!(convert_instr_to_binary(&".syscall 5".to_owned()).unwrap(),  0xF405_u16);
        assert_eq!(convert_instr_to_binary(&"JAL $r5, $r6".to_owned()).unwrap(),  0xFB80_u16);
    }


    #[test]
    #[should_panic]
    fn test_convert_invalid_instr_to_binary() {
        convert_instr_to_binary(&"INVALID  $r0, $zero, $r1".to_owned()).unwrap();
    }


    #[test]
    #[should_panic]
    fn test_convert_invalid_register_to_binary() {
        convert_instr_to_binary(&"ADD  $r0, $r9, $r1".to_owned()).unwrap();
    }


    #[test]
    fn test_file_bios() {
        let mut lines:Vec<String> = get_line_vector("test_files/test_file_bios.asm");
        lines = lines.into_iter().filter(|line| !line.is_empty()).collect();
        validate_assembly_lines(&lines).unwrap();

        lines = substitute_pseudoinstrs(&lines);
        let label_table = generate_label_table(&lines).unwrap();

        lines = substitute_labels(&lines, &label_table);

        let mut assembled_lines = Vec::new();
        for line in lines {
            assembled_lines.push(convert_instr_to_binary(&line).unwrap());
        }

        assert_eq!(assembled_lines[2], 0x280B);
        assert_eq!(assembled_lines[3], 0x6800);
    }
}

