module top;
generate
assign q = d;
endgenerate
endmodule
// expected -----
module top;
  generate
    assign q = d;
  endgenerate
endmodule
