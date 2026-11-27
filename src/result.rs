pub type Result<T> = core::result::Result<T, &'static str>;
/*
 * なんだこれ、、、Resultをcoreクレートのものを使うようにしているだけ？？
 * no_std環境ではstd::result::Resultではなくcore::result::Resultを使う
 */