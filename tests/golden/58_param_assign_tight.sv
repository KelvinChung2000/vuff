module top;
  sub#(8) u1(a, b);
endmodule
// expected -----
module top;
  sub #(8) u1 (a, b);
endmodule
