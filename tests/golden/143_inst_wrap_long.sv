module top;
  long_module_name u_inst_long_name (.clk(clk), .rst_n(rst_n),
    .data_in(data_in), .data_out(data_out));
  short u_short(.a(a));
endmodule
// expected -----
module top;
  long_module_name u_inst_long_name (
    .clk(clk),
    .rst_n(rst_n),
    .data_in(data_in),
    .data_out(data_out)
  );
  short u_short (.a(a));
endmodule
