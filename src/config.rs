use nom::{
    character::is_alphanumeric,
    character::complete::newline,
    character::complete::alphanumeric1,
    combinator::opt
};

use std::collections::HashMap;
use std::sync::Mutex;
use core::fmt::Debug;

lazy_static! {
    /// Stores the current state of all signals
    pub static ref STATE: Mutex<HashMap<String, u8>> = Mutex::new(HashMap::new());
}


/// Lookup (Truth) table for mapping input signals to output signals
/// only stores data on mapping, signal values are stored in STATE
#[derive(Debug, Eq, PartialEq)]
pub struct LUT {
    inputs: Vec<Var>,
    output: Var,
    mappings: HashMap<Vec<u8>, u8>
}


impl LUT {

    pub fn new(inputs: Vec<&str>, output: &str, mappings: Vec<&str>) -> LUT {

        let mut lut = LUT{
            inputs: inputs.into_iter().map(|x| Var::new(x.to_string())).collect(),
            output: Var::new(output.to_string()),
            mappings: HashMap::new()
        };
        for line in mappings {
            let kv: Vec<&str> = line.split_whitespace().collect();
            let k = kv[0].to_string()
                .chars()
                .map(|c| c.to_digit(2).unwrap() as u8).collect();
            let v = isize::from_str_radix(kv[1],2).unwrap() as u8;
            lut.mappings.insert(k, v);
        }

        lut
    }

    /// executes the LUT, setting the output signal based on current input
    fn exec(self) {

        let mut signals: Vec<u8> = vec!();
        for var in self.inputs {
            match STATE.lock().unwrap().get(&var.name) { // TODO: .lock().unwrap() as a macro possible?
                Some(&val) => signals.push(val),
                None => panic!("var '{}' was not initialized", var.name)
            };
        }

        match self.mappings.get(&signals) {
            Some(&v) => {
                STATE.lock().unwrap().insert(self.output.name, v);
            },
            None => {}
        };
    }
}


/// Direct mapping of input signal to output signal based on clock or other hardware
/// Rarely used in design, usually just maps signals to start at end of cycle
#[derive(Debug, Eq, PartialEq)]
pub struct Register {
    input: Var,
    output: Var,
    signal: String,
    control: Var,
    init: u8 // 0 = lo, 1 = hi, 2 = don't care, 3 = unknown
}

impl Register {

    pub fn new(input: &str,
               output: &str,
               clock: Option<(&str, &str)>,
               init: Option<char>) -> Register {

        // TODO: default global clock for latch
        let mut signal = "re";
        let mut control = "NIL";
        let mut start: u8 = 3;
        match clock {
            Some(clk) => {
                signal = clk.0;
                control = clk.1;
            }
            None => {}
        };
        match init {
            Some(i) => { start = i.to_digit(10).unwrap() as u8; }
            None => {}
        };

        Register{
            input: Var::new(input),
            output: Var::new(output),
            signal: signal.to_string(),
            control: Var::new(control),
            init: start
        }
    }

    fn exec(self) {
        // TODO: handle varying clock triggers if possible
        let &i = STATE.lock().unwrap().get(&self.input.name).unwrap();
        STATE.lock().unwrap().insert(self.output.name, i);
    }
}

/// Basic signal in design, holds only metadata while value is in STATE
#[derive(Debug, Eq, PartialEq)]
pub struct Var  {
    name: String,
}

impl Var {

    pub fn new<S>(name: S) -> Var where S: Into<String> {
        let n: String = name.into();
        STATE.lock().unwrap().insert(n.clone(), 0);
        Var{name: n}
    }

}

/// enumeration of possible Basic Logic Elements (BLE)
#[derive(Debug, Eq, PartialEq)]
pub enum Element {
    LUT(LUT),
    Register(Register)
}

/// Holds complete model representation
pub struct Model {
    name: String,
    inputs: Vec<Var>,
    outputs: Vec<Var>,
    elements: Vec<Element>
}

impl Model {
    pub fn new<S>(name: S,
                  inputs: Vec<Var>,
                  outputs: Vec<Var>,
                  elements: Vec<Element>) -> Model where S: Into<String> {
        Model{
            name: name.into(),
            inputs,
            outputs,
            elements
        }
    }
}

/// Entry for getting FPGA configuration
pub struct Config {
    models: Vec<Model>
}


impl Config {
    pub fn new(blif: &str) -> Self {
        Config{models: Config::parse_blif(blif)}
    }

    /// parses blif-formatted data into comprising models
    pub fn parse_blif(mut input: &str) -> Vec<Model> {

        let mut models: Vec<Model> = vec!();
        let mut res;

        while input.len() > 0 {
            res = opt(get_model)(input).unwrap();
            input = res.0;
            match res.1 {
                Some(m) => models.push(m),
                None => { input = garbage_line(input).unwrap().0 }
            };
        }
        models
    }
}


// TODO:
// - add parsing for inner models [x] (there are no inner modules since design is flattened)
// - form a graph structure representing order of true dependencies of different blocks [x] (yosys already does this)
// - implement LUT function for giving output on given input [x]
// eval loop for executing configuration [ ]
// a single cycle should run all LUTs (now CLBs) and IOBs [ ]
// - create IOB logic for r/w memory [ ]
// output pins for drawing to screen [ ]

// Each Var needs to hold its bit value [x]
// assume single clock for now

// never use traits for object instantiation, self-referencing traits are a bitch



named!(
    get_model_name<&str, &str>,
    do_parse!(
        name: preceded!(tag!(".model "), is_not!(" \n")) >>
        newline >>
        (name)
    )
);

#[test]
fn test_get_model_name() {
    assert_eq!(get_model_name(".model counter\n"), Ok(("", "counter")));
}

named!(
    get_inputs<&str, Vec<Var>>,
    do_parse!(
        alt!(tag!(".inputs ") | tag!(".inputs")) >>
        inputs: separated_list0!(tag!(" "), is_not!(" \n")) >>
        newline >>
        (inputs.into_iter().map(|x| Var::new(x.to_string())).collect())
    )
);

#[test]
fn test_get_inputs() {
    assert_eq!(get_inputs(".inputs in0 in1 in2\n"), Ok(("", vec!(Var{name:"in0".to_string()},
                                                               Var{name:"in1".to_string()},
                                                               Var{name:"in2".to_string()}))));
}

named!(
    get_outputs<&str, Vec<Var>>,
    do_parse!(
        alt!(tag!(".outputs ") | tag!(".outputs")) >>
        outputs: separated_list0!(tag!(" "), is_not!(" \n")) >>
        newline >>
        (outputs.into_iter().map(|x| Var::new(x.to_string())).collect())
    )
);

#[test]
fn test_get_outputs() {
    assert_eq!(get_outputs(".outputs out0 out1 out2\n"), Ok(("", vec!(Var{name:"out0".to_string()},
                                                               Var{name:"out1".to_string()},
                                                               Var{name:"out2".to_string()}))));
}

named!(
    get_lut<&str, Element>,
    do_parse!(
        tag!(".names ") >>
        io: separated_list0!(tag!(" "), alphanumeric1) >>
        newline >>
        lut: separated_list0!(tag!("\n"), is_a!(" 01-")) >>
        newline >>
        (Element::LUT(LUT::new(io[0 .. io.len() -1].to_vec(), io[io.len()-1], lut)))
    )
);

#[test]
fn test_get_lut() {
    let mut lut = Element::LUT(LUT::new(vec!("out0","out1","out2"), "return0", vec!("011 1", "100 1")));
    assert_eq!(get_lut(".names out0 out1 out2 return0\n011 1\n100 1\n.names"), Ok((".names", lut)));
}

named!(
    get_clock<&str, (&str, &str)>,
    do_parse!(
        tag!(" ") >>
        signal: alt!(tag!("fe") | tag!("re") | tag!("ah") | tag!("al") | tag!("as")) >>
        tag!(" ") >>
        control: is_not!(" \n") >>
        ((signal, control))
    )
);


named!(
    get_reg<&str, Element>,
    do_parse!(
        tag!(".latch ") >>
        input: is_not!(" \n") >>
        tag!(" ") >>
        output: is_not!(" \n") >>
        clock: opt!(get_clock) >>
        opt!(complete!(tag!(" "))) >>
        init: opt!(one_of!("0123")) >>
        newline >>
        (Element::Register(Register::new(input, output, clock, init)))
    )
);

#[test]
fn test_get_reg() {
    let mut reg = Element::Register(Register::new("$0out[8:0][8]", "out[8]", Some(("re", "clock")), Some('2')));
    assert_eq!(get_reg(".latch $0out[8:0][8] out[8] re clock 2\n"), Ok(("", reg)));

    let mut reg = Element::Register(Register::new("$0out[8:0][8]", "out[8]", Some(("re", "clock")), None));
    assert_eq!(get_reg(".latch $0out[8:0][8] out[8] re clock\n"), Ok(("", reg)));

    let mut reg = Element::Register(Register::new("$0out[8:0][8]", "out[8]", None, None));
    assert_eq!(get_reg(".latch $0out[8:0][8] out[8]\n"), Ok(("", reg)));

    let mut reg = Element::Register(Register::new("$0out[8:0][8]", "out[8]", None, Some('2')));
    assert_eq!(get_reg(".latch $0out[8:0][8] out[8] 2\n"), Ok(("", reg)));
}

named!(
    get_model<&str, Model>,
    do_parse!(
        name: get_model_name >>
        inputs: get_inputs >>
        outputs: get_outputs >>
        elements: many0!(alt!(get_lut | get_reg)) >>
        (Model::new(name, inputs, outputs, elements))
    )
);


#[test]
fn test_get_model() {
    // TODO: Create full test with example model
    assert!(get_model(
r#".model toplevel
.inputs clock plain[0] plain[1] plain[2] plain[3]
.outputs cipher[0] cipher[1] cipher[2] cipher[3]
.names state[3] state[2] state[1] state[0] done
1000 1
.names state[3] state[0] mod.state[0] $abc$8433$n994 $0\state[3:0][0]
0001 1
0011 1
0101 1
0110 1
0111 1
1100 1
1101 1
1110 1
1111 1
.latch $0\out[255:0][222] out[222] re clock 2
.latch $0\out[255:0][223] out[223] re clock 2
.latch $0\out[255:0][224] out[224] re clock 2
.names mod.state[0] $abc$8433$n2278 $abc$8433$n1807 $abc$8433$n1806 $abc$8433$n2289
0011 1
0111 1
1100 1
1101 1
1110 1
1111 1"#).is_ok());
}


named!(
    garbage_line<&str, &str>,
    do_parse!(
        garbage: take_until!("\n") >>
        newline >>
        (garbage)
    )
);



#[test]
fn test_parse_blif() {
    // TODO: test complete parsing more thoroughly
    let mut blif = Config::parse_blif(
r#"
# Generated by Yosys 0.9 (git sha1 UNKNOWN, gcc 10.1.0 -march=x86-64 -mtune=generic -O2 -fno-plt -fPIC -Os)

.model toplevel
.inputs clock plain[0] plain[1] plain[2] plain[3]
.outputs cipher[0] cipher[1] cipher[2] cipher[3]
.names state[3] state[0] mod.state[0] $abc$8433$n994 $0\state[3:0][0]
0001 1
0011 1
.latch $0\out[255:0][233] out[233] re clock 2
.names mod.state[1] mod.state[0] $abc$8433$n993
10 1

.model toplevel2
.inputs clock plain[0] plain[1] plain[2] plain[3]
.outputs cipher[0] cipher[1] cipher[2] cipher[3]
.names state[3] state[2] state[1] state[0] done
1000 1
.names state[3] state[0] mod.state[0] $abc$8433$n994 $0\state[3:0][0]
0001 1
0011 1
.latch $0\out[255:0][229] out[229] re clock 2
.latch $0\out[255:0][230] out[230] re clock 2
.end
"#);

    if blif.len() != 2 {
        assert!(false, "wrong number models returned.");
    }
}