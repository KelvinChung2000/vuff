// config: line_width=100
module top;
  fifo #(.WIDTH(8)) u_fifo (.clk(clk), .rst_n(rst_n), .data_i(d_i), .data_o(d_o));
endmodule
// expected -----
module top;
  fifo #(.WIDTH(8)) u_fifo (.clk(clk), .rst_n(rst_n), .data_i(d_i), .data_o(d_o));
endmodule
