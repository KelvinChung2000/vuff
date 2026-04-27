module top;
  fifo #(.WIDTH(8), .DEPTH(16)) u_fifo (.clk(clk), .rst_n(rst_n),
    .push(push), .pop(pop), .data_i(data_i), .data_o(data_o));
endmodule
// expected -----
module top;
  fifo #(.WIDTH(8), .DEPTH(16)) u_fifo (
    .clk(clk),
    .rst_n(rst_n),
    .push(push),
    .pop(pop),
    .data_i(data_i),
    .data_o(data_o)
  );
endmodule
