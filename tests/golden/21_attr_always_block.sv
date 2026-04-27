module m (input wire [1:0] sel, output logic q);
always_comb begin
(* full_case, parallel_case *)
case (sel)
2'd0: q = 1'b0;
default: q = 1'b1;
endcase
end
endmodule
// expected -----
module m (input wire [1:0] sel, output logic q);
  always_comb begin
    (* full_case, parallel_case *)
    case (sel)
      2'd0: q = 1'b0;
      default: q = 1'b1;
    endcase
  end
endmodule
