use std::fs;
use std::path::Path;

use mithqal::Session;

fn eval(session: &mut Session, source: &str) -> String {
    let value = session.eval_source(source).unwrap();
    session.format_infix(value).unwrap()
}

fn string_source(path: &Path) -> String {
    format!("\"{}\"", path.display())
}

#[test]
fn string_functions_work() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "stringLength(\"abc\")"), "3");
    assert_eq!(eval(&mut session, "stringJoin(\"a\",\"b\")"), "\"ab\"");
    assert_eq!(
        eval(&mut session, "stringSplit(\"a,b\",\",\")"),
        "[\"a\",\"b\"]"
    );
    assert_eq!(
        eval(&mut session, "stringReplace(\"aba\",\"a\",\"x\")"),
        "\"xbx\""
    );
    assert_eq!(eval(&mut session, "stringTake(\"abcdef\",3)"), "\"abc\"");
    assert_eq!(eval(&mut session, "stringTake(\"abcdef\",-3)"), "\"def\"");
    assert_eq!(eval(&mut session, "stringDrop(\"abcdef\",3)"), "\"def\"");
    assert_eq!(eval(&mut session, "stringDrop(\"abcdef\",-3)"), "\"abc\"");
    assert_eq!(eval(&mut session, "stringContains(\"abc\",\"b\")"), "true");
    assert_eq!(eval(&mut session, "stringReverse(\"abc\")"), "\"cba\"");
    assert_eq!(eval(&mut session, "characters(\"ab\")"), "[\"a\",\"b\"]");
    assert_eq!(eval(&mut session, "characterCode(\"A\")"), "65");
    assert_eq!(eval(&mut session, "fromCharacterCode(65)"), "\"A\"");
    assert_eq!(eval(&mut session, "toString(1+2)"), "\"3\"");
}

#[test]
fn byte_array_functions_work() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "byteArrayLength(byteArray(1,2))"), "2");
    assert_eq!(
        eval(&mut session, "byteArrayToList(byteArray(1,2))"),
        "[1,2]"
    );
    assert_eq!(
        eval(&mut session, "byteArrayFromList([1,2])"),
        "byteArray(1,2)"
    );
    assert_eq!(
        eval(&mut session, "byteArrayToString(byteArray(97,98))"),
        "\"ab\""
    );
    assert_eq!(
        eval(&mut session, "stringToByteArray(\"ab\")"),
        "byteArray(97,98)"
    );
}

#[test]
fn association_functions_work() {
    let mut session = Session::new();
    assert_eq!(eval(&mut session, "associationLength(<|a->1,b->2|>)"), "2");
    assert_eq!(
        eval(&mut session, "associationKeys(<|a->1,b->2|>)"),
        "[a,b]"
    );
    assert_eq!(
        eval(&mut session, "associationValues(<|a->1,b->2|>)"),
        "[1,2]"
    );
    assert_eq!(eval(&mut session, "lookup(<|a->1|>,a)"), "1");
    assert_eq!(eval(&mut session, "lookup(<|a->1|>,b,9)"), "9");
    assert_eq!(eval(&mut session, "keyExistsQ(<|a->1|>,a)"), "true");
    assert_eq!(
        eval(&mut session, "associationMerge(<|a->1|>,<|b->2,a->3|>)"),
        "<|a->3,b->2|>"
    );
    assert_eq!(
        eval(&mut session, "associationKeyTake(<|a->1,b->2|>,[a])"),
        "<|a->1|>"
    );
    assert_eq!(
        eval(&mut session, "associationKeyDrop(<|a->1,b->2|>,[a])"),
        "<|b->2|>"
    );
    assert_eq!(
        eval(&mut session, "associationFromPairs([[a,1],[b,2]])"),
        "<|a->1,b->2|>"
    );
    assert_eq!(
        eval(&mut session, "associationToPairs(<|a->1,b->2|>)"),
        "[[a,1],[b,2]]"
    );
    assert_eq!(
        eval(&mut session, "associationMap(<|a->1|>,fn(k,v)=>v+1)"),
        "<|a->2|>"
    );
}

#[test]
fn file_text_csv_and_json_round_trip() {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(format!("mithqal-io-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let text_path = directory.join("a.txt");
    let csv_path = directory.join("a.csv");
    let json_path = directory.join("a.json");
    let tsv_path = directory.join("a.tsv");
    let text_import_path = directory.join("b.txt");
    let subdirectory = directory.join("sub");
    let mut session = Session::new();
    let text = string_source(&text_path);
    let csv = string_source(&csv_path);
    let json = string_source(&json_path);
    let tsv = string_source(&tsv_path);
    let text_import = string_source(&text_import_path);
    let subdir = string_source(&subdirectory);
    let directory_source = string_source(&directory);

    assert_eq!(
        eval(&mut session, &format!("writeString({text},\"hello\")")),
        "null"
    );
    assert_eq!(
        eval(&mut session, &format!("readString({text})")),
        "\"hello\""
    );
    assert_eq!(eval(&mut session, &format!("fileExistsQ({text})")), "true");
    assert_eq!(
        eval(
            &mut session,
            &format!("exportCsv({csv},[[\"a\",\"b\"],[\"c\",\"d\"]])")
        ),
        "null"
    );
    assert_eq!(
        eval(&mut session, &format!("importCsv({csv})")),
        "[[\"a\",\"b\"],[\"c\",\"d\"]]"
    );
    assert_eq!(
        eval(
            &mut session,
            &format!("exportJson({json},<|a->1,b->[true,\"x\"]|>)")
        ),
        "null"
    );
    assert_eq!(
        eval(&mut session, &format!("importJson({json})")),
        "<|\"a\"->1,\"b\"->[true,\"x\"]|>"
    );
    assert_eq!(
        eval(&mut session, &format!("createDirectory({subdir})")),
        "null"
    );
    assert_eq!(
        eval(&mut session, &format!("directoryName({text})")),
        format!("\"{}\"", directory.display())
    );
    assert_eq!(
        eval(&mut session, &format!("exportText({text_import},\"hi\")")),
        "null"
    );
    assert_eq!(
        eval(&mut session, &format!("importText({text_import})")),
        "\"hi\""
    );
    assert_eq!(
        eval(
            &mut session,
            &format!("exportTsv({tsv},[[\"a\",\"b\"],[\"c\",\"d\"]])")
        ),
        "null"
    );
    assert_eq!(
        eval(&mut session, &format!("importTsv({tsv})")),
        "[[\"a\",\"b\"],[\"c\",\"d\"]]"
    );
    assert_eq!(
        eval(
            &mut session,
            &format!("length(fileNames({directory_source}))")
        ),
        "6"
    );
    assert_eq!(
        eval(
            &mut session,
            &format!("fileNameJoin([{directory_source},\"a.txt\"])")
        ),
        format!("\"{}\"", text_path.display())
    );
    assert_eq!(eval(&mut session, &format!("deleteFile({text})")), "null");
    assert_eq!(eval(&mut session, &format!("fileExistsQ({text})")), "false");
    let _ = fs::remove_dir_all(&directory);
}
