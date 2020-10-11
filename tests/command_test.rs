use minirc::command::*;

#[test]
pub fn parsing_privmsg_works() {
    let test_str = ":Ranmaru!~ranmaru@2a02:908:13b2:5380:6c18:852b:8306:ac33 PRIVMSG ##rantestfoobazinga1337 :Foo! :D";
    let expected = Command::Privmsg(
        "Ranmaru".to_owned(),
        "##rantestfoobazinga1337".to_owned(),
        "Foo! :D".to_owned(),
    );
    let as_sent = String::from("PRIVMSG ##rantestfoobazinga1337 :Foo! :D\r\n");
    assert_eq!(Command::from(test_str), expected);
    assert_eq!(expected.to_string(), as_sent);
}

#[test]
pub fn parsing_notice_works() {
    let test_str = ":niven.freenode.net NOTICE * :*** Looking up your hostname...";
    let expected = Command::Notice(
        "niven.freenode.net".to_owned(),
        "*".to_owned(),
        "*** Looking up your hostname...".to_owned(),
    );
    let as_sent = String::from("NOTICE * :*** Looking up your hostname...\r\n");
    assert_eq!(Command::from(test_str), expected);
    assert_eq!(expected.to_string(), as_sent);
}

#[test]
pub fn parsing_ping_works() {
    let test_str = ":niven.freenode.net PING :pong me back";
    let expected = Command::Ping(":pong me back".to_owned());
    let as_sent = String::from("PING :pong me back\r\n");
    assert_eq!(Command::from(test_str), expected);
    assert_eq!(expected.to_string(), as_sent);
}

#[test]
pub fn parsing_pong_works() {
    let test_str = ":Ranmaru!~ranmaru@2a02:908:13b2:5380:6c18:852b:8306:ac33 PONG :pong pong pong";
    let expected = Command::Pong(":pong pong pong".to_owned());
    let as_sent = String::from("PONG :pong pong pong\r\n");
    assert_eq!(Command::from(test_str), expected);
    assert_eq!(expected.to_string(), as_sent);
}

#[test]
pub fn sending_join_and_part_works() {
    let single = Command::Join(vec!["##foo".to_owned()]);
    let multiple = Command::Join(vec![
        "##foo".to_owned(),
        "#bar".to_owned(),
        "##baz".to_owned(),
    ]);
    let expected = String::from("JOIN ##foo\r\n");
    let expected_mult = String::from("JOIN ##foo,#bar,##baz\r\n");
    assert_eq!(single.to_string(), expected);
    assert_eq!(multiple.to_string(), expected_mult);
}

#[test]
pub fn printing_works() {
    let privmsg = Command::Privmsg(
        "Ranmaru".to_owned(),
        "##foo".to_owned(),
        "Hello World!".to_owned(),
    );
    assert_eq!(
        privmsg.to_printable().unwrap(),
        String::from("<Ranmaru> Hello World!")
    );
}
