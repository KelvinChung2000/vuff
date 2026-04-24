module m;
always_comb begin
case (sel)
2'd0: x = cond ? a : b;
default: x = 0;
endcase
end
endmodule
// expected -----
module m;
  always_comb begin
    case (sel)
      2'd0: x = cond ? a : b;
      default: x = 0;
    endcase
  end
endmodule
