module m;
  logic q;
  always_ff @ ( posedge clk ) begin
    q <= d;
  end
endmodule
// expected -----
module m;
  logic q;
  always_ff @(posedge clk) begin
    q <= d;
  end
endmodule
