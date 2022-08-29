.fill 8
.fill -10 # comment
.fill 0x1E4
binary: .fill 0b00110100111
.fill 'a'
.fill '~'

text: .text "hello world!" # some text

array: .space 20 [100, 'a', 'b', 0xFF, 0b11101] # an array

ADD  $r0, $zero, $r1
NAND $r5, $r4,   $r3
branch: BEQ $r0, $r1, $r2

loop_start: ADDI $r0, $r1, 20 # <--- test label.
SW $r0, $r5, 0b001101
LW $r0, $r5, 0x0F
jump: JAL $r6, $r5 

LUI $r4, 0x1de

end: ADD $r1, $r2, $r3 # end of program

    .syscall 7

NOP
    label: NOP # test NOP instr

    LLI $r0, 20     
    MOVI $r1, 0x10e
