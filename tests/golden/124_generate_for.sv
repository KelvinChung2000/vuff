module top;
  generate
  for (genvar i = 0; i < N; i++) begin: g_loop
  assign out[i] = in[i];
  end
  endgenerate
endmodule
// expected -----
module top;
  generate
    for (genvar i = 0; i < N; i++) begin: g_loop
      assign out[i] = in[i];
    end
  endgenerate
endmodule
