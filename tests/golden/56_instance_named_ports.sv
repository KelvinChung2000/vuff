module top;
  sub u1(.a(x), .b(y), .c(z));
endmodule
// expected -----
module top;
  sub u1 (.a(x), .b(y), .c(z));
endmodule
