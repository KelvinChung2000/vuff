module top;
  sub  u1  (.clk(clk), .rst_n(rst_n));
  sub2  u2(a, b);
endmodule
// expected -----
module top;
  sub u1 (.clk(clk), .rst_n(rst_n));
  sub2 u2 (a, b);
endmodule
