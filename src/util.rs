pub fn is_ipv4(s: &String) -> bool
{
    //217.97.122.207
    if s.len() < 7 || s.len() > 15
    {
        return false;
    }

    fn check_octet(octet: &String) -> bool
    {
        match octet.len()
        {
            3 => {
                let mut chrs = octet.chars();
                let ch1 = chrs.next().unwrap();

                if !(ch1 == '1' || ch1 == '2') { return false; }
                if ch1 == '2'
                {
                    let ch2 = chrs.next().unwrap();
                    if !(ch2 == '0' || ch2 == '1' || ch2 == '2' || ch2 == '3' || ch2 == '4')
                    {
                        if ch2 == '5'
                        {
                            let ch3 = chrs.next().unwrap();
                            if !(ch3 == '0' || ch3 == '1' || ch3 == '2' || ch3 == '3' || ch3 == '4' || ch3 == '5')
                            {
                                return false;
                            }
                        }
                        else
                        {
                            return false;
                        }
                    }
                }
            },
            2 => {},
            1 => {},
            _ => return false
        }

        true
    }

    let mut octet = String::new();
    for ch in s.chars()
    {
        if ch == '.'
        {
            if !check_octet(&octet) { return false; }

            octet = String::new();
        }
        else
        {
            octet.push(ch);
        }
    }

    if !check_octet(&octet) { return false; }

    true
}