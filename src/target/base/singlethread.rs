use crate::arch::Arch;
use crate::target::base::*;
use crate::target::Target;

/// Core operations for single threaded targets
#[allow(clippy::type_complexity)]
pub trait SingleThread: Target {
    /// Resume execution on the target.
    ///
    /// `action` specifies how the target should be resumed (i.e:
    /// single-step vs. full continue).
    ///
    /// The `check_gdb_interrupt` callback can be invoked to check if GDB sent
    /// an Interrupt packet (i.e: the user pressed Ctrl-C). It's recommended to
    /// invoke this callback every-so-often while the system is running (e.g:
    /// every X cycles/milliseconds). Periodically checking for incoming
    /// interrupt packets is _not_ required, but it is _recommended_.
    ///
    /// # Implementation requirements
    ///
    /// These requirements cannot be satisfied by `gdbstub` internally, and must
    /// be handled on a per-target basis.
    ///
    /// ### Adjusting PC after a breakpoint is hit
    ///
    /// The [GDB remote serial protocol documentation](https://sourceware.org/gdb/current/onlinedocs/gdb/Stop-Reply-Packets.html#swbreak-stop-reason)
    /// notes the following:
    ///
    /// > On some architectures, such as x86, at the architecture level, when a
    /// > breakpoint instruction executes the program counter points at the
    /// > breakpoint address plus an offset. On such targets, the stub is
    /// > responsible for adjusting the PC to point back at the breakpoint
    /// > address.
    ///
    /// Omitting PC adjustment may result in unexpected execution flow and/or
    /// breakpoints not appearing to work correctly.
    fn resume(
        &mut self,
        action: ResumeAction,
        check_gdb_interrupt: &mut dyn FnMut() -> bool,
    ) -> Result<StopReason<<Self::Arch as Arch>::Usize>, Self::Error>;

    /// Read the target's registers.
    fn read_registers(
        &mut self,
        regs: &mut <Self::Arch as Arch>::Registers,
    ) -> Result<(), Self::Error>;

    /// Write the target's registers.
    fn write_registers(
        &mut self,
        regs: &<Self::Arch as Arch>::Registers,
    ) -> Result<(), Self::Error>;

    /// Read bytes from the specified address range.
    ///
    /// ### Handling non-fatal invalid memory reads
    ///
    /// If the requested address range could not be accessed (e.g: due to
    /// MMU protection, unhanded page fault, etc...), return `Ok(false)` to
    /// signal that the requested memory could not be read.
    ///
    /// As a reminder, `Err(Self::Error)` should only be returned if a memory
    /// read results in a **fatal** target error.
    fn read_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &mut [u8],
    ) -> Result<bool, Self::Error>;

    /// Write bytes to the specified address range.
    ///
    /// ### Handling non-fatal invalid memory writes
    ///
    /// If the requested address range could not be accessed (e.g: due to
    /// MMU protection, unhanded page fault, etc...), return `Ok(false)` to
    /// signal that the requested memory could not be written to.
    ///
    /// As a reminder, `Err(Self::Error)` should only be returned if a memory
    /// write results in a **fatal** target error.
    fn write_addrs(
        &mut self,
        start_addr: <Self::Arch as Arch>::Usize,
        data: &[u8],
    ) -> Result<bool, Self::Error>;
}
