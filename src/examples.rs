use std::fmt;

struct Structure(i32);

impl fmt::Display for Structure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { 
        write!(f, "{}", self.0)
    }

}


#[derive(Debug)]
struct Person<'a> {
    name: &'a str,
    age: i32
}


// Probably doesn't work because of generic
// impl fmt::Display for Person<'a> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error>{ 
//         write!(f, "{0}, {1} years old", self.name, self.age);
//     }
// }

#[derive(Debug)]
struct Point {
    x: f64,
    y: f64,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { 
        write!(f, "x: {}, y: {}", self.x, self.y)
    }
}

#[derive(Debug)]
struct Complex {
    real: f64, 
    imag: f64,
}

impl fmt::Display for Complex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { 
        write!(f, "{} + {}i", self.real, self.imag)
    }
}


struct List(Vec<i32>);

impl fmt::Display for List {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Extract the value using tuple indexing,
        // and create a reference to `vec`.
        let vec = &self.0;

        write!(f, "[")?;

        // Iterate over `v` in `vec` while enumerating the iteration
        // count in `count`.
        for (count, v) in vec.iter().enumerate() {
            // For every element except the first, add a comma.
            // Use the ? operator to return on errors.
            if count != 0 { write!(f, ", ")?; }
            write!(f, "{index}: {value}", index=count, value=v)?;
        }

        // Close the opened bracket and return a fmt::Result value.
        write!(f, "]")
    }
}

#[derive(Debug)]
struct Color {
    red: u8,
    green: u8,
    blue: u8,
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "RGB ({r}, {g}, {b}) 0x{r:02X}{g:02X}{b:02X}", 
            r=self.red, 
            g=self.green, 
            b=self.blue
        )
    }
}

#[derive(Debug)]
struct Matrix(f32, f32, f32, f32);

impl fmt::Display for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Matrix(a, b, c, d) = self;

        writeln!(f, "( {} {} )", a, b)?;
        write!(f, "( {} {} )", c, d)
    }

}

fn transpose(matrix: Matrix) -> Matrix {
    let Matrix(a, b, c, d) = matrix;

    Matrix(a, c, b, d)
}

#[derive(Debug)]
struct Rectangle {
    // A rectangle can be specified by where the top left and bottom right
    // corners are in space.
    top_left: Point,
    bottom_right: Point,
}

fn rect_area(rect: Rectangle) -> f64 {
    let Rectangle { top_left: Point{ x: left, y: top }, bottom_right: Point { x: right, y: bottom} } = rect;
    (right - left).abs() * (bottom - top).abs()
}

fn square(bottom_left: Point, size: f64) -> Rectangle {
    let Point { x: left, y: bottom } = bottom_left;

    Rectangle {
        top_left : Point { x: left, y: bottom + size},
        bottom_right: Point { x: left + size, y: bottom}
    }
}

fn main() {
    let name = "Roman";
    let age = 34;
    let roman = Person { name, age };


    println!("Debug:");
    println!("{:?}", roman);
    println!("");
    // println!("Display:");
    // println!("{}", roman);
    // println!("");
    println!("Structure: {}", Structure(42));

    let complex = Complex {
        real: 3.3, 
        imag: 7.2
    };

    println!("Display: {}", complex);
    println!("Debug: {:#?}", complex);

    let v = List(vec![1, 2, 3]);
    println!("{}", v);


    for color in [
        Color { red: 128, green: 255, blue: 90 },
        Color { red: 0, green: 3, blue: 254 },
        Color { red: 0, green: 0, blue: 0 },
    ].iter() {
        // Switch this to use {} once you've added an implementation
        // for fmt::Display.
        println!("{}", *color);
    }

    let matrix = Matrix(1.1, 1.2, 2.1, 2.2);
    println!("{:?}", matrix);
    
    println!("Matrix:\n{}", matrix);
    println!("Transpose:\n{}", transpose(matrix));
    

    let point: Point = Point { x: 10.3, y: 0.4 };
    println!("point coordinates: ({}, {})", point.x, point.y);

    let bottom_right = Point { x: 5.2, ..point };
    println!("second point: ({}, {})", bottom_right.x, bottom_right.y);

    let Point { x: left_edge, y: top_edge } = point;
    let rectangle = Rectangle {
        // struct instantiation is an expression too
        top_left: Point { x: left_edge, y: top_edge },
        bottom_right: Point { y: 2.4, ..bottom_right},
    };

    println!("Rect area: {:.2}", rect_area(rectangle));


    let square = square(point, 10f64);

    println!("Rect: {:#?}", square);
    println!("Rect area: {:.2}", rect_area(square));
}