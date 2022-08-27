use std::env;
use std::fs::OpenOptions;
use std::io::{ BufReader, BufRead };


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
    println!("{:?}", lines);
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
}
