# The Iridium Computer Assembler

## Introduction

The Iridium Assembler is based on an instruction set documented [here](https://user.eng.umd.edu/~blj/RiSC/RiSC-isa.pdf) by Professor Bruce Jacob for his series of lectures on Digital Computer Design at the University of Maryland in Fall 2000, many thanks to him.

The instruction set architecture (ISA) works off of 16-bit instructions and a word-size of 2 bytes, with 64kb (2^16) of memory locations of 16 bits each. There are 8 registers labelled *\$zero* and *\$r0-\$r6*, each addressed with 3 bits. The register *$zero* is read-only and always contains the value 0.

The ISA used is a RISC architecture with only 8 instructions and 4 pseudo-instructions outlined later in this file. This is enough to ensure that the ISA is Turing-complete, with a few helpful utilities.

## Instructions

Instructions fall into 3 categories: RRR-type, RRI-type, and RI-type, which are formatted as follows:

![Type formats](format_diagrams.svg)

The instruction set is laid out in the table below:

| Mnem | Opcode | Format    | Example           | Description                       |
|------|--------|-----------|-------------------|-----------------------------------|
| ADD  | 000    | RRR-Type  | ADD $r0 $r1 $r2   | Ra = Rb + Rc                      |
| ADDI | 001    | RRI-Type  | ADDI $r0 $zero 27 | Ra = Rb + Imm                     |
| NAND | 010    | RRR-Type  | NAND $r0 $r0 $r1  | Ra = ¬(Rb & Rc)                   |
| LUI  | 011    | RI-Type   | LUI $r0 0x1DE     | Bits 7-10 of Ra = Imm             |
| SW   | 100    | RRI-Type  | SW $r0 $r1 50     | Value of RAM at Rb + Imm = Ra     |
| LW   | 101    | RRI-Type  | LW $r0 $r1 .loc   | Ra = value of RAM at Rb + Imm     |
| BEQ  | 110    | RRI-Type  | BEQ $r0 $r1 @loop | If Ra == Rb, branch to Imm        |
| JAL  | 111    | RRI-Type* | JAL $r7 @pos      | Branch to addr in Rb, Ra = PC + 1 |

*The immediate in the JAL instruction is left blank

### Formatting and Validating Instructions

The general formal for a line of assembly code is:

`label:<whitespace>opcode<whitespace>field0, field1, field2<whilespace> #comments`

These can be validated using some regular expressions for each of the instruction formats:
```
RRR-Type: ^([a-zA-Z]+:)?([[:blank:]]*)(ADD|NAND)[[:blank:]]+(((\$(zero|r[0-6])),)(?2))(?4)(?6)(?2)(#(?2)[[:print:]]+)?$

RRI-Type: ^([a-zA-Z]+:)?([[:blank:]]*)(ADDI|SW|LW|BEQ|JAL)[[:blank:]]+(((\$(zero|r[0-6])),)(?2))(?4)(0*((-|\+)?[0-9]+|0b[01]+|0x[[:xdigit:]]+))(?2)(#(?2)[[:print:]]+)?$

RI-Type: ^([a-zA-Z]+:)?([[:blank:]]*)LUI(?2)(((\$(zero|r[0-6])),)(?2))(0*([0-9]+|0b[01]+|0x[[:xdigit:]]+))(?2)(#(?2)[[:print:]]+)?$
```

Further constraints on the instructions are that the 7 bit immediates cannot be outside the range -64 to 63, and the 10 bit immediates cannot be outside the range 0 to 0x03FF or 0 to 1023.

We also must take the format of the immediates into account as they may be in decimal form with no prefix, in binary form with the 0b prefix, or in hex form with the 0x prefix, and ensure that these are also in the range.


### Syscalls

Syscalls are made by writing the instruction `syscall [code]` in the same way as a normal instruction. This will be substituted by the assembler with a `JAL` command to a location in the OS section of RAM to invoke the relevant subroutine. The value stored in **$r6** will by taken as an argument for this if one is required. The available syscall instructions are as follows:
 - **print_char**: outputs the value in **$r6** as a character
 - **print_str**: outputs characters in RAM starting at the address in **$r6** and stopping when it reaches **\0**
 - **print_int**: outputs the value in **$r6** as a decimal integer
 - **print_hex**: outputs the value in **$r6** as a hexadecimal integer
 - **input_int**: awaits input from the user in decimal format and stores it in **$r6**
 - **input_char**: awaits input from the user as a character and stores it in **$r6**
 - **halt**: terminates program execution and freezes
 - **error**: terminates program and freezes, outputting **ERROR**


## Pseudo-Instructions

The program may also contain the following directives for the assembler:
 - **NOP**: the processor does nothing this cycle, and is replaced by the instruction `ADD $zero $zero $zero` which clearly does nothing but takes 1 cycle to do.
 - **LLI**: formatted as `LLI $Ra Imm` ORs the 6-bit immediate operand into the register $Ra and is replaced by `ADD $rX, imm6` upon compilation. This is useful when used in combination with LUI to load a full 16 bit value into a register.
 - **MOVI**: formatted as `MOVI $rX, Imm`, MOVI is shorthand for LUI + LLI and takes a 16-bit operand and puts it into the specified register. This instruction assembles to 2 instructions, and can therefore confuse jumping to numerical addresses, to labels should be used if at all possible.
 - **.fill**: formatted as `.fill Imm` tells the assembler to place a 16-bit immediate value here instead of an instruction. If it is used with a label address instead of an immediate, such as `.fill end`, then the address of the label will be inserted. It can also take a character in the form `'char'`, such as `'a'` and converts it to its ASCII representation.
 - **.space**: formatted as `.space Imm [Values]`, it is replaced by a number of `.fill` instructions equal to the immediate operand which fills the locations with the value in Values at that index, and 0x0000 if index > len(values).
 - **.text**: formatted as `.text "some string"`, it does the same as `.space` except converts each character in the string to its ASCII representation and uses those as the values to insert plus a null terminator **\0** to insert into a .space the same length as the string + 1.

These are each validated differently:
-  `NOP` is simply required to match the regex `^([[:blank:]]*)([a-zA-Z]+:)?(?1)NOP(?1)(#[[:print:]]*)?$`.
-  `LLI` should match the regex `^([[:blank:]]*)([a-zA-Z]+:)?(?1)LLI(?1)(\$r[0-6]),(?1)(0*([0-9]+|0b[01]+|0x[[:xdigit:]]+))(?1)(#[[:print:]]*)?$` and have an immediate between 0 and 63.
-  `MOVI` should match the regex `^([[:blank:]]*)([a-zA-Z]+:)?(?1)(MOVI)(?1)(\$r[0-6]),(?1)(0*((-|\+)?[0-9]+|0b[01]+|0x[[:xdigit:]]+))(?1)(#[[:print:]]*)?$` and have an immediate between -32,768 and 32,767.
-  `.fill` should match the regex `^([[:blank:]]*)([a-zA-Z]+:)?(?1).fill(?1)((0*((-|\+)?[0-9]+|0b[01]+|0x[[:xdigit:]]+))|'[[:ascii:]]')(?1)(#[[:print:]]*)?$` and have any non-character immediate be between -32,768 and 32,767.
-  `.space` should match the regex `^([[:blank:]]*)([a-zA-Z]+:)(?1).space(?1)(0*([0-9]+|0b[01]+|0x[[:xdigit:]]+))(?1)\[(('[[:ascii:]]'|(0*((-|\+)?[0-9]+|0b[01]+|0x[[:xdigit:]]+))),(?1))*\](?1)(#[[:print:]]*)?$` and have any non-character immediate be between -32,768 and 32,767 and have the size of the space be >= the size of the array.
-  `.text` should match the regex `^([[:blank:]]*)([a-zA-Z]+:)(?1).text(?1)(?1)"([[:ascii:]]+)"(?1)(#[[:print:]]*)?$`


## Process of Assembly

The assembly code will be processed in 3 passes of the input file:
 1. **Read Phase**: The file is scanned and turned into a vector of lines.
 2. **Validation Phase**: The vector has each line validated and the programmer is informed if any invalid code is detected.
 3. **Pseudo Phase**: Any pseudo-instructions and syscalls are found and the appropriate substitutions are made.
 4. **Label Table Phase**: Any labels are found and inserted into a table of the name of the label and the location in memory it refers to.
 5. **Label Substitution Phase** Labels are changed to their proper values using the table from the previous phase.
 6. **Binary Generation Phase** The final vector of lines in converted into binary and written to the output file. 


## Notes

In order to subtract numbers, the programmer should flip the bits of the value to subtract and add 1. Flipping the bits can be done by NANDing the value with itself and adding 1 by using the ADDI instruction. 

Multiplication can be achieved by repeated addition, bit-testing, and left-shifting by 1 (the same as doubling).


## Current State of Development

 - [ ] Opening and reading input file to generate vector of lines
 - [ ] Validating lines vector
   - [ ] Validating RRR-Type instructions
   - [ ] Validating RRI-Type instructions
   - [ ] Validating RI-Type instructions
   - [ ] Validating instruction pseudo-instructions
   - [ ] Validating data pseudo-instructions
   - [ ] Validating syscalls
 - [ ] Pseudo-Instruction substitution
   - [ ] Find pseudo-instructions in the vector
   - [ ] Determine correct substitution(s) to make
   - [ ] Make substitutions
 - [ ] Label table generation
   - [ ] Find all labels
   - [ ] Generate table of names and locations of labels
 - [ ] Label substitution
   - [ ] Find all references to labels in the vector
   - [ ] Make substitution if the address will fit in the space allowed for the immediate
   - [ ] Add instructions to push *\$r6* to the stack, move address into *\$r6*, use address, and restore *\$r6*
 - [ ] Convert instructions to binary
 - [ ] Write final binary to file 
