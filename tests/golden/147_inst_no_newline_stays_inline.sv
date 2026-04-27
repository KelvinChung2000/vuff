// config: line_width=60
module top;
  long_module_name u_inst_long_name (.clk(clk), .rst_n(rst_n), .data_in(data_in), .data_out(data_out));
endmodule
// expected -----
module top;
  long_module_name u_inst_long_name (.clk(clk), .rst_n(rst_n), .data_in(data_in), .data_out(data_out));
endmodule
