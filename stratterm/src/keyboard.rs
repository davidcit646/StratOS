// Basic keysym to char mapping for ASCII (MVP - no xkbcommon)
// Wayland keysym values from linux/input-event-codes.h

pub fn keysym_to_char(keysym: u32) -> Option<char> {
    match keysym {
        // Printable ASCII (0x20-0x7E)
        0x0020 => Some(' '),
        0x0021 => Some('!'),
        0x0022 => Some('"'),
        0x0023 => Some('#'),
        0x0024 => Some('$'),
        0x0025 => Some('%'),
        0x0026 => Some('&'),
        0x0027 => Some('\''),
        0x0028 => Some('('),
        0x0029 => Some(')'),
        0x002A => Some('*'),
        0x002B => Some('+'),
        0x002C => Some(','),
        0x002D => Some('-'),
        0x002E => Some('.'),
        0x002F => Some('/'),
        0x0030 => Some('0'),
        0x0031 => Some('1'),
        0x0032 => Some('2'),
        0x0033 => Some('3'),
        0x0034 => Some('4'),
        0x0035 => Some('5'),
        0x0036 => Some('6'),
        0x0037 => Some('7'),
        0x0038 => Some('8'),
        0x0039 => Some('9'),
        0x003A => Some(':'),
        0x003B => Some(';'),
        0x003C => Some('<'),
        0x003D => Some('='),
        0x003E => Some('>'),
        0x003F => Some('?'),
        0x0040 => Some('@'),
        0x0041 => Some('A'),
        0x0042 => Some('B'),
        0x0043 => Some('C'),
        0x0044 => Some('D'),
        0x0045 => Some('E'),
        0x0046 => Some('F'),
        0x0047 => Some('G'),
        0x0048 => Some('H'),
        0x0049 => Some('I'),
        0x004A => Some('J'),
        0x004B => Some('K'),
        0x004C => Some('L'),
        0x004D => Some('M'),
        0x004E => Some('N'),
        0x004F => Some('O'),
        0x0050 => Some('P'),
        0x0051 => Some('Q'),
        0x0052 => Some('R'),
        0x0053 => Some('S'),
        0x0054 => Some('T'),
        0x0055 => Some('U'),
        0x0056 => Some('V'),
        0x0057 => Some('W'),
        0x0058 => Some('X'),
        0x0059 => Some('Y'),
        0x005A => Some('Z'),
        0x005B => Some('['),
        0x005C => Some('\\'),
        0x005D => Some(']'),
        0x005E => Some('^'),
        0x005F => Some('_'),
        0x0060 => Some('`'),
        0x0061 => Some('a'),
        0x0062 => Some('b'),
        0x0063 => Some('c'),
        0x0064 => Some('d'),
        0x0065 => Some('e'),
        0x0066 => Some('f'),
        0x0067 => Some('g'),
        0x0068 => Some('h'),
        0x0069 => Some('i'),
        0x006A => Some('j'),
        0x006B => Some('k'),
        0x006C => Some('l'),
        0x006D => Some('m'),
        0x006E => Some('n'),
        0x006F => Some('o'),
        0x0070 => Some('p'),
        0x0071 => Some('q'),
        0x0072 => Some('r'),
        0x0073 => Some('s'),
        0x0074 => Some('t'),
        0x0075 => Some('u'),
        0x0076 => Some('v'),
        0x0077 => Some('w'),
        0x0078 => Some('x'),
        0x0079 => Some('y'),
        0x007A => Some('z'),
        0x007B => Some('{'),
        0x007C => Some('|'),
        0x007D => Some('}'),
        0x007E => Some('~'),
        _ => None,
    }
}

// Control keys (return as special sequences)
pub fn keysym_to_control(keysym: u32) -> Option<Vec<u8>> {
    match keysym {
        0xFF0D => Some(vec![b'\r']), // Enter/Return
        0xFF08 => Some(vec![0x7F]), // Backspace
        0xFF09 => Some(vec![b'\t']), // Tab
        0xFF51 => Some(vec![0x1B, b'[', b'D']), // Left arrow (ESC[D)
        0xFF52 => Some(vec![0x1B, b'[', b'A']), // Up arrow (ESC[A)
        0xFF53 => Some(vec![0x1B, b'[', b'C']), // Right arrow (ESC[C)
        0xFF54 => Some(vec![0x1B, b'[', b'B']), // Down arrow (ESC[B)
        0xFF50 => Some(vec![0x1B, b'[', b'H']), // Home (ESC[H)
        0xFF57 => Some(vec![0x1B, b'[', b'F']), // End (ESC[F)
        0xFF55 => Some(vec![0x1B, b'[', b'5', b'~']), // Page Up (ESC[5~)
        0xFF56 => Some(vec![0x1B, b'[', b'6', b'~']), // Page Down (ESC[6~)
        0xFF1B => Some(vec![0x1B]), // Escape
        0xFFE1 => Some(vec![]), // Left Shift (no output)
        0xFFE2 => Some(vec![]), // Right Shift (no output)
        0xFFE3 => Some(vec![]), // Left Ctrl (no output)
        0xFFE4 => Some(vec![]), // Right Ctrl (no output)
        0xFFE9 => Some(vec![]), // Left Alt (no output)
        0xFFEA => Some(vec![]), // Right Alt (no output)
        _ => None,
    }
}

pub fn is_control_key(keysym: u32) -> bool {
    matches!(keysym, 
        0xFF0D | 0xFF08 | 0xFF09 | 0xFF51 | 0xFF52 | 0xFF53 | 0xFF54 | 
        0xFF50 | 0xFF57 | 0xFF55 | 0xFF56 | 0xFF1B | 0xFFE1 | 0xFFE2 | 
        0xFFE3 | 0xFFE4 | 0xFFE9 | 0xFFEA
    )
}
