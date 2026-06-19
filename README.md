# rxv6
Migration and backup of rxv6 from my previous academic account.

This is a small educational small operating system inspired by xv6. To run this using qemu,

## In branch 'main':
Command to try the operating system with virt-io:

```
make run
```
Commands to try the operating system with stdio:
```
make build
qemu-system-riscv64 -machine virt -nographic -bios none -kernel target/riscv64gc-unknown-none-elf/release/rxv6 2>&1
```
(Use the same commands for branch: 'qemu_virt-io')

## In branch 'qemu_no-graphic'
Primitive state of the repository, you can try it out using just:
```
make run
```
