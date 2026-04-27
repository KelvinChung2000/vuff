module m (input wire a, input wire b, output logic q);
always_comb begin
if (a) begin
if (b) q = 1'b1;
else q = 1'b0;
end else begin
q = 1'b0;
end
end
endmodule
// expected -----
module m (input wire a, input wire b, output logic q);
  always_comb begin
    if (a) begin
      if (b) q = 1'b1;
      else q = 1'b0;
    end else begin
      q = 1'b0;
    end
  end
endmodule
