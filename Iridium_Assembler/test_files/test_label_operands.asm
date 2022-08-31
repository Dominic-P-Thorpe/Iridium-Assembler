start: ADD $r0, $r1, $r2
NOP

move: MOVI $r6, @start
JAL $r5,$r6

ADDI $r0, $zero, @move
text: .text "Something here which is very, very long and takes up a lot of space..."
after_text: MOVI $r6, @text
MOVI $r5, @after_text