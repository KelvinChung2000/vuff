module top;
  fifo  #(.WIDTH(8) , .DEPTH(16))   u1  ( .clk(clk), .rst(rst));
  buf3  #(8)   u2  (in, out);
endmodule
// expected -----
module top;
  fifo #(.WIDTH(8), .DEPTH(16)) u1 (.clk(clk), .rst(rst));
  buf3 #(8) u2 (in, out);
endmodule
