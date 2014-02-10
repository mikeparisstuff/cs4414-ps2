struct Node { 
	val: int,
	tail: LinkedList
}

type LinkedList = Option<~Node>;


fn construct_list(n: int, x: int) -> LinkedList {
	match n {
		0 => { None }
		_ => { Some(~Node{val: x, tail: construct_list(n-1, x+1)})}
	}
}

fn print_list(ls: LinkedList) -> ~str {
	match ls {
		None => { ~"" },
		Some(s) => format!("{:d}, {:s}", s.val, print_list(s.tail))
	}
}

trait Map {
	fn mapr(&mut self, fn(int) -> int);
}

impl Map for LinkedList {
	fn mapr(&mut self, f: fn(int) -> int) {
		match( *self ) {
			None => { }
			Some(ref mut current) => {
				let (port, chan) : (Port<int>, Chan<int>) = Chan::new();
				let val = current.val; // Can't capture current
				spawn(proc() { chan.send(f(val)); });
				current.tail.mapr(f); // why here?
				current.val = port.recv();

				// current.val = f(current.val);
				// current.tail.mapr(f);
			}
		}
	}
}

fn expensive_inc(n: int) -> int {
	let mut a = 1;
	println!("starting inc: {:d}", n);
	for _ in range(0, 10000) {
		for _ in range(0, 100000) {
			a = a + 1;
		}
	}
	println!("finished inc: {:d} ({:d})", n, a);
	n + 1
}

fn inc(n: int) -> int { n + 1 }
fn double(n: int) -> int { n * 2 }

fn main() {
	let mut l: LinkedList = construct_list(4,10);
	l.mapr(expensive_inc);


	// l10.mapr(inc);
	// l10.mapr(double);
	println!("List: {:s}", print_list(l));
}