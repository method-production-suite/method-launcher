//! What a Mach-O file is built for (docs/spec.md §4.2, macOS): whether it runs on this Mac (it
//! has this Mac's own slice), and whether it is universal (exactly Intel and Apple silicon,
//! the suite's Mac builds from 2026-10-19). The launcher reads the headers itself, so the
//! check doesn't need developer tools on the Mac. Only the start of a
//! file is needed: [`HEADER_BYTES`] covers a fat header with every slice an app could carry.

/// How many bytes from the start of a file [`classify`] needs at most.
pub const HEADER_BYTES: usize = 4096;

/// `CPU_TYPE_ARM64` from `<mach/machine.h>`.
pub const CPU_ARM64: u32 = 0x0100_000C;

/// `CPU_TYPE_X86_64` from `<mach/machine.h>`.
pub const CPU_X86_64: u32 = 0x0100_0007;

/// This Mac's own CPU type, as the launcher was built for it.
pub const fn host_cpu() -> Option<u32> {
    if cfg!(target_arch = "aarch64") {
        Some(CPU_ARM64)
    } else if cfg!(target_arch = "x86_64") {
        Some(CPU_X86_64)
    } else {
        None
    }
}

/// What the start of a file says it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Not a Mach-O file (a picture, a plist, a script). Not checked further.
    NotMachO,
    /// A Mach-O whose only code is arm64: thin, or fat with arm64 slices alone.
    Arm64Only,
    /// A Mach-O with other code in it; the CPU types found, in file order.
    Other(Vec<u32>),
    /// Starts like a Mach-O but is cut short or malformed. Fails the check.
    Damaged,
}

fn u32_at(bytes: &[u8], at: usize, big_endian: bool) -> Option<u32> {
    let word: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
    Some(if big_endian { u32::from_be_bytes(word) } else { u32::from_le_bytes(word) })
}

/// Classifies a file from its first bytes (up to [`HEADER_BYTES`]).
pub fn classify(head: &[u8]) -> Kind {
    let Some(magic) = u32_at(head, 0, true) else { return Kind::NotMachO };
    match magic {
        // Thin files, stored little-endian on Apple's platforms: 64-bit, then 32-bit.
        0xCFFA_EDFE | 0xCEFA_EDFE => match u32_at(head, 4, false) {
            Some(CPU_ARM64) if magic == 0xCFFA_EDFE => Kind::Arm64Only,
            Some(cpu) => Kind::Other(vec![cpu]),
            None => Kind::Damaged,
        },
        // Big-endian thin files never hold arm64.
        0xFEED_FACF | 0xFEED_FACE => u32_at(head, 4, true).map_or(Kind::Damaged, |cpu| Kind::Other(vec![cpu])),
        // Fat files, big-endian: 32-bit (20-byte entries) and 64-bit (32-byte entries).
        0xCAFE_BABE | 0xCAFE_BABF => {
            let Some(count) = u32_at(head, 4, true) else { return Kind::Damaged };
            // Java class files share 0xCAFEBABE; their next word is a version, 45 or more.
            if magic == 0xCAFE_BABE && count >= 45 {
                return Kind::NotMachO;
            }
            let entry = if magic == 0xCAFE_BABE { 20 } else { 32 };
            let mut cpus = Vec::new();
            for i in 0..count as usize {
                match u32_at(head, 8 + i * entry, true) {
                    Some(cpu) => cpus.push(cpu),
                    None => return Kind::Damaged,
                }
            }
            if !cpus.is_empty() && cpus.iter().all(|&cpu| cpu == CPU_ARM64) { Kind::Arm64Only } else { Kind::Other(cpus) }
        }
        _ => Kind::NotMachO,
    }
}

/// True when a file with these first bytes passes the old Apple-silicon-only check: either
/// not Mach-O at all, or Mach-O with arm64 code only.
pub fn passes(head: &[u8]) -> bool {
    matches!(classify(head), Kind::NotMachO | Kind::Arm64Only)
}

/// The CPU types a Mach-O holds: one for a thin file, each slice's for a fat one; empty for a
/// file that isn't Mach-O; `None` for one that is cut short or malformed.
pub fn cpus(head: &[u8]) -> Option<Vec<u32>> {
    match classify(head) {
        Kind::NotMachO => Some(Vec::new()),
        Kind::Arm64Only => Some(match u32_at(head, 0, true) {
            Some(0xCAFE_BABE | 0xCAFE_BABF) => {
                let count = u32_at(head, 4, true).unwrap_or(0) as usize;
                vec![CPU_ARM64; count]
            }
            _ => vec![CPU_ARM64],
        }),
        Kind::Other(cpus) => Some(cpus),
        Kind::Damaged => None,
    }
}

/// Whether a file runs on a Mac with this CPU: not Mach-O at all, or a Mach-O with a slice for
/// it.
pub fn runs_on(head: &[u8], cpu: u32) -> bool {
    cpus(head).is_some_and(|cpus| cpus.is_empty() || cpus.contains(&cpu))
}

/// Whether a Mach-O is exactly universal: an Intel slice and an Apple silicon slice, nothing
/// else (the suite's Mac builds, SUITE-STANDARDS.md §3).
pub fn universal(head: &[u8]) -> bool {
    cpus(head).is_some_and(|mut cpus| {
        cpus.sort_unstable();
        cpus == [CPU_X86_64, CPU_ARM64]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const X86_64: u32 = CPU_X86_64;

    fn thin64(cpu: u32) -> Vec<u8> {
        let mut b = vec![0xCF, 0xFA, 0xED, 0xFE];
        b.extend(cpu.to_le_bytes());
        b.extend([0; 24]);
        b
    }

    fn fat(cpus: &[u32]) -> Vec<u8> {
        let mut b = vec![0xCA, 0xFE, 0xBA, 0xBE];
        b.extend((cpus.len() as u32).to_be_bytes());
        for cpu in cpus {
            b.extend(cpu.to_be_bytes());
            b.extend([0; 16]);
        }
        b
    }

    #[test]
    fn thin_arm64_passes() {
        assert_eq!(classify(&thin64(CPU_ARM64)), Kind::Arm64Only);
    }

    #[test]
    fn thin_intel_fails() {
        assert_eq!(classify(&thin64(X86_64)), Kind::Other(vec![X86_64]));
        assert!(!passes(&thin64(X86_64)));
    }

    #[test]
    fn universal_fails_and_arm64_only_fat_passes() {
        assert_eq!(classify(&fat(&[X86_64, CPU_ARM64])), Kind::Other(vec![X86_64, CPU_ARM64]));
        assert_eq!(classify(&fat(&[CPU_ARM64])), Kind::Arm64Only);
    }

    #[test]
    fn empty_or_cut_short_fat_fails() {
        assert!(!passes(&fat(&[])));
        let mut cut = fat(&[CPU_ARM64, CPU_ARM64]);
        cut.truncate(20);
        assert_eq!(classify(&cut), Kind::Damaged);
    }

    #[test]
    fn other_files_are_not_mach_o() {
        assert_eq!(classify(b"#!/bin/sh\necho hi\n"), Kind::NotMachO);
        assert_eq!(classify(b"<?xml"), Kind::NotMachO);
        assert_eq!(classify(&[]), Kind::NotMachO);
        // A Java class file: 0xCAFEBABE, then minor and major version.
        assert_eq!(classify(&[0xCA, 0xFE, 0xBA, 0xBE, 0x00, 0x00, 0x00, 0x34]), Kind::NotMachO);
    }

    #[test]
    fn runs_on_this_mac_by_its_own_slice() {
        // An Apple-silicon-only app doesn't run on an Intel Mac; a universal one runs on both.
        assert!(runs_on(&thin64(CPU_ARM64), CPU_ARM64));
        assert!(!runs_on(&thin64(CPU_ARM64), CPU_X86_64));
        assert!(runs_on(&fat(&[X86_64, CPU_ARM64]), CPU_X86_64));
        assert!(runs_on(&fat(&[X86_64, CPU_ARM64]), CPU_ARM64));
        assert!(runs_on(b"#!/bin/sh", CPU_X86_64), "not Mach-O: nothing to check");
        let mut cut = fat(&[X86_64, CPU_ARM64]);
        cut.truncate(20);
        assert!(!runs_on(&cut, CPU_ARM64), "damaged fails");
    }

    #[test]
    fn universal_means_exactly_both() {
        assert!(universal(&fat(&[X86_64, CPU_ARM64])));
        assert!(universal(&fat(&[CPU_ARM64, X86_64])));
        assert!(!universal(&fat(&[CPU_ARM64])));
        assert!(!universal(&thin64(CPU_ARM64)));
        assert!(!universal(&fat(&[X86_64, CPU_ARM64, 12])));
        assert!(!universal(b"#!/bin/sh"));
        assert_eq!(host_cpu().is_some(), cfg!(any(target_arch = "aarch64", target_arch = "x86_64")));
    }

    #[test]
    fn thirty_two_bit_fails() {
        let mut b = vec![0xCE, 0xFA, 0xED, 0xFE];
        b.extend(12u32.to_le_bytes());
        assert!(!passes(&b));
    }
}
