.section .text
.align 8
.global _vector_table
_vector_table:
    j _trap_sync           /* 0  User software interrupt */
    j _trap_s_soft         /* 1  Supervisor software interrupt */
    j _trap_sync           /* 2  Reserved */
    j _trap_sync           /* 3  Machine software interrupt */
    j _trap_sync           /* 4  User timer interrupt */
    j _trap_s_timer        /* 5  Supervisor timer interrupt */
    j _trap_sync           /* 6  Reserved */
    j _trap_sync           /* 7  Machine timer interrupt */
    j _trap_sync           /* 8  User external interrupt */
    j _trap_s_ext          /* 9  Supervisor external interrupt */
    j _trap_sync           /* 10 Reserved */
    j _trap_sync           /* 11 Machine external interrupt */

.global _trap_sync
.global _trap_s_soft
.global _trap_s_timer
.global _trap_s_ext

/* Synchronous exceptions always go to BASE+0 regardless of vectored mode.
   Interrupts go to BASE + 4*scause. */
_trap_sync:
    j _trap_entry

_trap_s_soft:
    j _trap_entry

_trap_s_timer:
    j _trap_entry

_trap_s_ext:
    j _trap_entry
