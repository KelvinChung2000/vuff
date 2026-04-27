module top;
  generate
  if (W == 8) begin: g_w8
  assign q = d;
  end else begin
  assign q = 0;
  end
  endgenerate
endmodule
// expected -----
module top;
  generate
    if (W == 8) begin: g_w8
      assign q = d;
    end else begin
      assign q = 0;
    end
  endgenerate
endmodule
