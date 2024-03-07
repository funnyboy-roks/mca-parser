/// Mod into positive, i.e. -1 % 16 == 15
#[macro_export]
macro_rules! positive_mod {
    ($ex: expr, $num: expr) => {{
        let ex = $ex;
        let num = $num;
        ((ex % num) + num) % num
    }};
}
