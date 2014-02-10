//
// gash.rs
//
// Starting code for PS2
// Running on Rust 0.9
//
// University of Virginia - cs4414 Spring 2014
// Weilin Xu, David Evans
// Version 0.4
//

extern mod extra;
extern mod native;

use std::{io, run, os};
use std::io::buffered::BufferedReader;
use std::io::stdio::stdin;
use std::run::{Process, ProcessOptions};
use std::io::{File, Open, Read, Write};
use std::path::posix::Path;
use std::libc::{fileno, fopen, pid_t, c_int};
use std::libc;
use std::io::signal::{Listener, Interrupt};
use extra::getopts;

struct Shell {
    cmd_prompt: ~str,
    working_directory: Path,
    history: ~[~str],
    output_redirects: ~[~str]
}

impl Shell {
    fn new(prompt_str: &str) -> Shell {
        Shell {
            cmd_prompt: prompt_str.to_owned(),
            working_directory: Path::new(os::getcwd()),
            history: ~[],
            output_redirects: ~[]
        }
    }
    
    fn run(&mut self) {
        let mut stdin = BufferedReader::new(stdin());
        // println!("PWD: {}", self.working_directory.display() );
        loop {
            print(self.cmd_prompt);
            io::stdio::flush();
            
            let line = stdin.read_line().unwrap();
            let cmd_line = line.trim().to_owned();

            // Add this command to the history
            self.add_to_history(cmd_line.clone());

            // for &x in ins.iter() {
            //     println!("Elem {}", x);
            // }

            // Check to see if there are any output redirects
            // if cmd_line.contains(">") {
            //     let mut split : ~[&str] = cmd_line.split('>').collect();
            //     split.remove(0);
            //     let mut split2 = ~[];
            //     for x in split.iter() {
            //         split2.push(x.to_owned());
            //     }
            //     self.output_redirects = split2;
            // }

            let program = cmd_line.splitn(' ', 1).nth(0).expect("no program");
            
            match program {
                ""      =>  { continue; }
                "exit"  =>  { return; }
                _       =>  { self.run_cmdline(cmd_line); }
            }
        }
    }

    fn get_output_redirects(&mut self, cmd_line: &str) -> ~[&str] {
        /*
        *   Get the output redirect filenames designated by >
        */
        let mut output_files : ~[&str] = ~[];
        if cmd_line.contains(">") {
            let mut files: ~[&str] = cmd_line.split('>').collect();
            files.remove(0);
            output_files = files.clone();
        }
        return output_files;
    }

    fn get_input_redirect(&mut self, cmd_line: &str) -> ~[&str] {
        let mut input_files : ~[&str] = ~[];
        if cmd_line.contains("<") {
            let mut files: ~[&str] = cmd_line.split('<').collect();
            files.remove(0);
            for elem in files.iter() {
                if elem.contains(">") {
                    let mut outs: ~[&str] = elem.split('>').collect();
                    let inputf : &str = outs[0];
                    input_files.push(inputf.trim());
                } else {
                    input_files.push(elem.trim());
                }
            }
        }
        input_files
    }

    fn get_pipes(&mut self, cmd_line: &str) -> ~[&str] {
        let mut pipes : ~[&str] = ~[];
        if cmd_line.contains("|") {
            let mut temp : ~[&str] = cmd_line.split('|').collect();
            temp.remove(0);
            pipes = temp.clone();
        }
        pipes
    }

    fn add_to_history(&mut self, cmd_line: ~str) {
        if cmd_line != ~"" {
            self.history.push(cmd_line);
        }
    }

    fn print_history(&mut self) {
        let mut count = 0;
        for x in self.history.iter() {
            println!("  {} {}",count, *x);
            count += 1;
        }
    }

    fn should_run_in_background(&mut self, cmd_line: &str) -> bool {
        // Check to see if we should be running in the background
        let mut run_in_background : bool = false;
        if cmd_line.contains("&") {
            run_in_background = true;
        }
        run_in_background
    }

    fn change_directory(&mut self, cmd_line: &str) {
        let mut argv: ~[~str] =
            cmd_line.split(' ').filter_map(|x| if x != "" { Some(x.to_owned()) } else { None }).to_owned_vec();

        if argv.len() > 0 {
            // Remove the "cd" argument
            argv.remove(0);
            if argv.len() > 0 {
                self.working_directory.push(argv.remove(0));
            } else {
                self.working_directory = os::homedir().unwrap()
            }
            // println!("Changing directory to: {}", self.working_directory.display());
            os::change_dir(&self.working_directory);
            // println!("CWD: {}", self.working_directory.display());
        }
    }
    
    fn run_cmdline_in_background(&mut self, cmd_line: &str) {
        let mut argv: ~[~str] =
            cmd_line.split(' ').filter_map(|x| if x != "" { Some(x.to_owned()) } else { None }).to_owned_vec(); 

        // Remove & from argument list
        let mut ind = -1;
        let mut count = 0;
        for x in argv.iter() {
            if *x == ~"&" {
                ind = count;
            }
            count += 1;
        }
        argv.remove(ind);

        // let (port, chan): (Port<int>, Chan<int>) = Chan::new();

        println("[1]");

        if( argv[0] == ~"cd" ) {
            if argv.len() > 0 {
                argv.remove(0);
                if argv.len() > 0 {
                    self.working_directory.push(argv.remove(0));
                } else {
                    self.working_directory = os::homedir().unwrap()
                }
                let working_dir = self.working_directory.clone();
                spawn(proc() { 
                    os::change_dir(&working_dir);
                } );
            }
        } else if( argv[0] == ~"history" ) {
            let hist : ~[~str] = self.history.clone();

            spawn( proc() { 
                let mut count = 0;
                for x in hist.iter() {
                    println!("  {} {}",count, *x);
                    count += 1;
                }});
        } else if argv.len() > 0 {
            let prog: ~str = argv.remove(0);
            let args: ~[~str] = argv.clone();
            // fn call(f: |~str, ~[~str]|) { |prog, args| println!("Program={}, args={}", prog, args); }
            // let closure = |prog, args| println!("Program={}, args={}", prog, args);

            spawn( proc() {

                fn cmd_exists(cmd_path: &str) -> bool {
                    let ret = run::process_output("which", [cmd_path.to_owned()]);
                    return ret.expect("exit code error.").status.success();
                }

                for x in args.iter() {
                    print(x.clone())
                }
                // println("");
                if cmd_exists(prog) {
                    run::process_status(prog, args);
                } else {
                    println!("{:s}: command not found", prog);
                }
                // chan.send(1);
            });
        }      
        // println!("[{:d}] Done",port.recv());

        // println("Done Executing Task");
    }

    fn run_cmdline(&mut self, cmd_line: &str) {

        // Check to see if we should be running in the background
        let run_in_background : bool = self.should_run_in_background(cmd_line.clone());
        let output_redirects = self.get_output_redirects(cmd_line);
        let input_redirects : ~[&str] = self.get_input_redirect(cmd_line);

        if run_in_background {
            self.run_cmdline_in_background(cmd_line.clone());
        } 
        else if input_redirects.len() > 0 {
            let temp_split : ~[&str] = cmd_line.split('<').collect();
            let cmd_line_sans_redirects = temp_split[0];
            let mut argv: ~[~str] = 
                cmd_line_sans_redirects.split(' ').filter_map(|x| if x != "" { Some(x.to_owned()) } else { None }).to_owned_vec();

            if argv.len() > 0 {
                let program: ~str = argv.remove(0);
                self.run_cmd_with_input_redirect(program, argv, input_redirects);
            }
        }
        else if output_redirects.len() > 0 {
            // Need to first create the output files if they do not exist
            for &file in output_redirects.iter() {
                // Trim the filename to get rid of weird whitespace bugs
                let path: Path = Path::new(file.trim());
                if !path.is_file() {
                    File::create(&path);
                }
            }

            // Get the program and args.. We do not want the things after > as that will cause errors
            let temp_split : ~[&str] = cmd_line.split('>').collect();
            let cmd_line_sans_redirects = temp_split[0];
            let mut argv: ~[~str] =
                cmd_line_sans_redirects.split(' ').filter_map(|x| if x != "" { Some(x.to_owned()) } else { None }).to_owned_vec();


            if argv.len() > 0 {
                let program: ~str = argv.remove(0);
                self.run_cmd_with_output_redirect(program, argv, output_redirects);
            }
        }
        else {
            let mut argv: ~[~str] =
                cmd_line.split(' ').filter_map(|x| if x != "" { Some(x.to_owned()) } else { None }).to_owned_vec();

            if( argv[0] == ~"cd" ) {
                self.change_directory(cmd_line);
            } else if( argv[0] == ~"history" ) {
                self.print_history();
            } else if argv.len() > 0 {
                let program: ~str = argv.remove(0);
                self.run_cmd(program, argv);
            }
        }
    }

    fn run_cmd_with_output_redirect(&mut self, program: &str, argv: &[~str], output_files: ~[&str]) {
        if self.cmd_exists(program) {
            let write_to_file : &str = output_files[output_files.len()-1].trim();
            let f = match native::io::file::open(&write_to_file.to_c_str(), Open, Write) {
                Ok(f)   => f,
                Err(e)  => { fail!("Could not open output file") }
            };
            let file_descriptor = f.fd();

            let mut process = run::Process::new(program, argv, run::ProcessOptions {
                env: None,
                dir: None,
                in_fd: Some(unsafe { libc::dup(libc::STDIN_FILENO) }),
                out_fd: Some(file_descriptor),
                err_fd: Some(unsafe { libc::dup(libc::STDERR_FILENO) })
            });

            // process.finish();
        } else {
            println!("{:s}: command not found", program);
        }
    }

    fn run_cmd_with_input_redirect(&mut self, program: &str, argv: &[~str], input_files: ~[&str]) {
        if self.cmd_exists(program) {
            let read_from_file : &str = input_files[input_files.len()-1].trim();
            let f = match native::io::file::open(&read_from_file.to_c_str(), Open, Read) {
                Ok(f)   => f,
                Err(e)  => fail!("Problem opening file")
            };
            let file_descriptor = f.fd();

            let mut process = run::Process::new(program, argv, run::ProcessOptions {
                env: None,
                dir: None,
                in_fd: Some(file_descriptor),
                out_fd: Some(unsafe { libc::dup(libc::STDOUT_FILENO) }),
                err_fd: Some(unsafe { libc::dup(libc::STDERR_FILENO) })
            });
        } else {
            println!("{:s}: command not found", program);   
        }
    }
    
    fn run_cmd(&mut self, program: &str, argv: &[~str]) {
        // println!("Program: {:s}", program);
        // print("Has args: ");

        // for x in argv.iter() {
        //     print(x.clone())
        // }

        // println("");
        if self.cmd_exists(program) {
            run::process_status(program, argv);
        } else {
            println!("{:s}: command not found", program);
        }
    }
    
    fn cmd_exists(&mut self, cmd_path: &str) -> bool {
        // println!("Checking if command {:s} exists", cmd_path);
        let ret = run::process_output("which", [cmd_path.to_owned()]);
        return ret.expect("exit code error.").status.success();
    }
}

fn get_cmdline_from_args() -> Option<~str> {
    /* Begin processing program arguments and initiate the parameters. */
    let args = os::args();
    
    let opts = ~[
        getopts::optopt("c")
    ];
    
    let matches = match getopts::getopts(args.tail(), opts) {
        Ok(m) => { m }
        Err(f) => { fail!(f.to_err_msg()) }
    };
    
    if matches.opt_present("c") {
        let cmd_str = match matches.opt_str("c") {
                                                Some(cmd_str) => {cmd_str.to_owned()}, 
                                                None => {~""}
                                              };
        return Some(cmd_str);
    } else {
        return None;
    }
}

fn handle_interrupt() {
    let mut l = Listener::new();
    l.register(Interrupt);

    let l_copy = l;
    spawn(proc() {
        loop {
            match l_copy.port.recv() {
                Interrupt => continue,
                _         => ()
            }
        }
    });
}

fn main() {
    handle_interrupt();

    let opt_cmd_line = get_cmdline_from_args();
    match opt_cmd_line {
        Some(cmd_line) => { println!("Command: {:s}", cmd_line); Shell::new("").run_cmdline(cmd_line) },
        None           => { Shell::new("gash > ").run() }
    }
}
