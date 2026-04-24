module top;
  sub u1(a), u2(b), u3(c);
endmodule
// expected -----
module top;
  sub u1 (a), u2 (b), u3 (c);
endmodule
