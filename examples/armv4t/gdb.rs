use armv4t_emu::{reg, Memory};
use gdbstub::arch;
use gdbstub::target::base::{ResumeAction, StopReason};
use gdbstub::target::ext::breakpoint::{BreakOp, WatchKind};
use gdbstub::target::{base, ext, Target};

use crate::emu::{Emu, Event};

impl Target for Emu {
    type Arch = arch::arm::Armv4t;
    type Error = &'static str;

    fn base_ops(&mut self) -> base::BaseOps<Self::Arch, Self::Error> {
        base::BaseOps::SingleThread(self)
    }

    fn sw_breakpoint(&mut self) -> ext::SwBreakpointExt<Self> {
        self
    }

    fn hw_watchpoint(&mut self) -> Option<ext::HwWatchpointExt<Self>> {
        Some(self)
    }
}

impl base::SingleThread for Emu {
    fn resume(
        &mut self,
        action: ResumeAction,
        check_gdb_interrupt: &mut dyn FnMut() -> bool,
    ) -> Result<StopReason<u32>, Self::Error> {
        let event = match action {
            ResumeAction::Step => match self.step() {
                Some(e) => e,
                None => return Ok(StopReason::DoneStep),
            },
            ResumeAction::Continue => {
                let mut cycles = 0;
                loop {
                    if let Some(event) = self.step() {
                        break event;
                    };

                    // check for GDB interrupt every 1024 instructions
                    cycles += 1;
                    if cycles % 1024 == 0 && check_gdb_interrupt() {
                        return Ok(StopReason::GdbInterrupt);
                    }
                }
            }
        };

        Ok(match event {
            Event::Halted => StopReason::Halted,
            Event::Break => StopReason::HwBreak,
            Event::WatchWrite(addr) => StopReason::Watch {
                kind: WatchKind::Write,
                addr,
            },
            Event::WatchRead(addr) => StopReason::Watch {
                kind: WatchKind::Read,
                addr,
            },
        })
    }

    fn read_registers(
        &mut self,
        regs: &mut arch::arm::reg::ArmCoreRegs,
    ) -> Result<(), &'static str> {
        let mode = self.cpu.mode();

        for i in 0..13 {
            regs.r[i] = self.cpu.reg_get(mode, i as u8);
        }
        regs.sp = self.cpu.reg_get(mode, reg::SP);
        regs.lr = self.cpu.reg_get(mode, reg::LR);
        regs.pc = self.cpu.reg_get(mode, reg::PC);
        regs.cpsr = self.cpu.reg_get(mode, reg::CPSR);

        Ok(())
    }

    fn write_registers(&mut self, regs: &arch::arm::reg::ArmCoreRegs) -> Result<(), &'static str> {
        let mode = self.cpu.mode();

        for i in 0..13 {
            self.cpu.reg_set(mode, i, regs.r[i as usize]);
        }
        self.cpu.reg_set(mode, reg::SP, regs.sp);
        self.cpu.reg_set(mode, reg::LR, regs.lr);
        self.cpu.reg_set(mode, reg::PC, regs.pc);
        self.cpu.reg_set(mode, reg::CPSR, regs.cpsr);

        Ok(())
    }

    fn read_addrs(&mut self, start_addr: u32, data: &mut [u8]) -> Result<bool, &'static str> {
        for (addr, val) in (start_addr..).zip(data.iter_mut()) {
            *val = self.mem.r8(addr)
        }
        Ok(true)
    }

    fn write_addrs(&mut self, start_addr: u32, data: &[u8]) -> Result<bool, &'static str> {
        for (addr, val) in (start_addr..).zip(data.iter().copied()) {
            self.mem.w8(addr, val)
        }
        Ok(true)
    }
}

impl ext::breakpoint::SwBreakpoint for Emu {
    fn update_sw_breakpoint(&mut self, addr: u32, op: BreakOp) -> Result<bool, &'static str> {
        match op {
            BreakOp::Add => self.breakpoints.push(addr),
            BreakOp::Remove => {
                let pos = match self.breakpoints.iter().position(|x| *x == addr) {
                    None => return Ok(false),
                    Some(pos) => pos,
                };
                self.breakpoints.remove(pos);
            }
        }

        Ok(true)
    }
}

impl ext::breakpoint::HwWatchpoint for Emu {
    fn update_hw_watchpoint(
        &mut self,
        addr: u32,
        op: BreakOp,
        kind: WatchKind,
    ) -> Result<bool, &'static str> {
        match op {
            BreakOp::Add => {
                match kind {
                    WatchKind::Write => self.watchpoints.push(addr),
                    WatchKind::Read => self.watchpoints.push(addr),
                    WatchKind::ReadWrite => self.watchpoints.push(addr),
                };
            }
            BreakOp::Remove => {
                let pos = match self.watchpoints.iter().position(|x| *x == addr) {
                    None => return Ok(false),
                    Some(pos) => pos,
                };

                match kind {
                    WatchKind::Write => self.watchpoints.remove(pos),
                    WatchKind::Read => self.watchpoints.remove(pos),
                    WatchKind::ReadWrite => self.watchpoints.remove(pos),
                };
            }
        }

        Ok(true)
    }
}
