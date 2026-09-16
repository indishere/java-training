int num = 5;
num = 5 + 7; 
num = num * 2;

float dec;
dec = 54.98f;

double biggerDecimal = 9879098.78998;
char s = 's';
String str = "Hello everybody!";
boolean b = true;
boolean c = false;
// num = 5.7;

System.out.println(num);
System.out.println("Hello World.");
System.out.println(5 + 7);
System.out.println(12 - 7);
System.out.println(6 * 7); // multiply
System.out.println(5 / 4); // divide
System.out.println(5 % 4); // remainder
System.out.println("hello\n" + "how are you");

int x = 5;
int xsq = x * x;
int y = (3 * xsq) - (2 * x) + 5;
int yy = (3 * (x * x)) - (2 * x) + 5;
System.out.println("the ans is: " + y);

/*
Scanner scan = new Scanner(System.in);
System.out.println("what is your name?");
String name = scan.nextLine();
System.out.println("Hello " + name);
System.out.println("How old are you?");
int age = scan.nextInt();
System.out.println("ur " + age + " yrsold");
*/

int a = 5;
int e = 10;

if (a > e) {
    System.out.println(">");
} else {
    System.out.println("<");
}

if (a == e) {
    System.out.println("equal");
}

if (a <= e) {
    System.out.println("less than or equal");
}

if (a >= e) {
    System.out.println("Greater than or equal");
}

if (a != e) {
    System.out.println("not equal");
}