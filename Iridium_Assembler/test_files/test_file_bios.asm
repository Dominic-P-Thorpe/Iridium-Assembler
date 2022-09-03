MOVI $r0, 0xFFFF          # EOF code
MOVI $r1, @end            # address of where to branch to when $r6 = EOF
ADDI $r2, $zero, 0        # index of current instruction
MOVI $r3, @loop           # index of start of instruction writing loop

loop: BEQ $r6, $r0, $r1 
SW $r6, $r2, 0
ADDI $r2, $r2, 1

JAL $zero, $r1

end: .syscall 6
