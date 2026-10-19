//! Is a file an Apple-silicon-only Mach-O? (docs/spec.md §4.4 step 4.) The launcher reads the
//! headers itself, so the check doesn't need developer tools on the Mac. Only the start of a
//! file is needed: [`HEADER_BYTES`] covers a fat header with every slice an app could carry.

/// How many bytes from the start of a file [`classify`] needs at most.
pub const HEADER_BYTES: usize = 4096;

/// `CPU_TYPE_ARM64` from `<mach/machine.h>`.
pub const CPU_ARM64: u32 = 0x0100_000C;

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

/// True when a file with these first bytes passes the architecture check: either not Mach-O
/// at all, or Mach-O with arm64 code only.
pub fn passes(head: &[u8]) -> bool {
    matches!(classify(head), Kind::NotMachO | Kind::Arm64Only)
}

#[cfg(test)]
mod tests {
    use super::*;

    const X86_64: u32 = 0x0100_0007;

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
    fn thirty_two_bit_fails() {
        let mut b = vec![0xCE, 0xFA, 0xED, 0xFE];
        b.extend(12u32.to_le_bytes());
        assert!(!passes(&b));
    }
}
