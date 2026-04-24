module top;
  sub #(.WIDTH(8), .DEPTH(4)) u1(.clk(clk));
endmodule
// expected -----
module top;
  sub #(.WIDTH(8), .DEPTH(4)) u1 (.clk(clk));
endmodule
